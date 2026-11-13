//! macOS and Windows engine: ffmpeg and deno sidecars bundled via `externalBin`, plus the
//! yt-dlp folder bundled as a resource.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Mutex;
use std::time::Duration;

use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};

use super::args::{self, ToolPaths};
use super::metadata;
use super::progress::{OutputLine, ProgressTracker};
use super::*;
use crate::t;

const METADATA_TIMEOUT: Duration = Duration::from_secs(180);
const KILL_GRACE: Duration = Duration::from_secs(3);
/// The single-file yt-dlp unpacks itself on every run (10 s+), so the folder build is used.
/// Tauri puts resources in `Contents/Resources` on macOS and next to the binary elsewhere.
const YT_DLP_DIST: &str = "yt-dlp-dist";

pub struct YtDlpEngine {
    tools: ToolPaths,
    running: Mutex<HashSet<u32>>,
}

impl YtDlpEngine {
    /// Tauri places sidecars next to the main binary without the target suffix
    /// (`Contents/MacOS/ffmpeg`, `ffmpeg.exe`, `target/debug/ffmpeg` in development).
    pub fn from_sidecars() -> Result<Self, EngineError> {
        let exe = std::env::current_exe().map_err(|source| EngineError::Spawn {
            tool: "vana",
            source,
        })?;
        let dir = exe.parent().ok_or_else(|| {
            EngineError::Message(t!("App folder not found.", "Uygulama klasörü bulunamadı."))
        })?;
        Ok(Self::from_dir(dir))
    }

    pub fn from_dir(dir: &Path) -> Self {
        let bin = |name: &str| dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
        let yt_dlp_name = format!("yt-dlp{}", std::env::consts::EXE_SUFFIX);
        let yt_dlp = [
            dir.join(YT_DLP_DIST),
            dir.join("../Resources").join(YT_DLP_DIST),
        ]
        .map(|d| d.join(&yt_dlp_name))
        .into_iter()
        .find(|p| p.exists())
        .unwrap_or_else(|| dir.join(YT_DLP_DIST).join(&yt_dlp_name));
        Self {
            tools: ToolPaths {
                yt_dlp,
                ffmpeg: bin("ffmpeg"),
                deno: bin("deno"),
                aria2c: find_aria2c(),
            },
            running: Mutex::new(HashSet::new()),
        }
    }

    fn command(&self, program: &Path) -> Command {
        let mut cmd = Command::new(program);
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env("PYTHONIOENCODING", "utf-8")
            .env("PYTHONUTF8", "1")
            .kill_on_drop(true);
        // yt-dlp is a PyInstaller process that starts ffmpeg; running it in its own
        // process group lets us stop them all together.
        #[cfg(unix)]
        cmd.process_group(0);
        #[cfg(windows)]
        {
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        cmd
    }

    fn spawn_yt_dlp(&self, args: Vec<String>) -> Result<Child, EngineError> {
        let child = self
            .command(&self.tools.yt_dlp)
            .args(args)
            .spawn()
            .map_err(|source| EngineError::Spawn {
                tool: "yt-dlp",
                source,
            })?;
        if let Some(pid) = child.id() {
            self.running.lock().unwrap().insert(pid);
        }
        Ok(child)
    }

    fn forget(&self, pid: Option<u32>) {
        if let Some(pid) = pid {
            self.running.lock().unwrap().remove(&pid);
        }
    }

    /// Runs a yt-dlp download with progress and cancel support; returns the final file path.
    async fn transfer(
        &self,
        args: Vec<String>,
        mut tracker: ProgressTracker,
        on_progress: &ProgressSink,
        mut cancel: CancelSignal,
    ) -> Result<PathBuf, EngineError> {
        let mut child = self.spawn_yt_dlp(args)?;
        let pid = child.id();
        let stdout = child.stdout.take().expect("stdout is piped");
        let stderr = child.stderr.take().expect("stderr is piped");

        let stderr_task = tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            let mut collected = String::new();
            while let Ok(Some(line)) = lines.next_line().await {
                collected.push_str(&line);
                collected.push('\n');
            }
            collected
        });

        let mut lines = BufReader::new(stdout).lines();
        let mut final_path = None;
        let mut canceled = *cancel.borrow();

        while !canceled {
            tokio::select! {
                line = lines.next_line() => match line {
                    Ok(Some(line)) => match tracker.parse_line(&line) {
                        OutputLine::Progress(update) => on_progress(update),
                        OutputLine::FinalPath(path) => final_path = Some(PathBuf::from(path)),
                        OutputLine::Other => {}
                    },
                    _ => break,
                },
                changed = cancel.changed() => {
                    canceled = changed.is_err() || *cancel.borrow();
                }
            }
        }

        if canceled {
            terminate(&mut child).await;
            self.forget(pid);
            stderr_task.abort();
            return Err(EngineError::Canceled);
        }

        let status = child.wait().await.map_err(|source| EngineError::Spawn {
            tool: "yt-dlp",
            source,
        })?;
        self.forget(pid);
        let stderr = stderr_task.await.unwrap_or_default();

        match final_path {
            // With `--ignore-errors` a subtitle error can fail the exit code while the file is fine.
            Some(path) if path.exists() => Ok(path),
            _ if status.success() => Err(EngineError::Message(t!(
                "The download finished but the output file was not found.",
                "İndirme bitti ama çıktı dosyası bulunamadı."
            ))),
            _ => Err(EngineError::Message(friendly_error(&stderr))),
        }
    }

    async fn tool_version(&self, name: &str, path: &Path, flag: &str) -> ToolStatus {
        let output = self.command(path).arg(flag).output().await;
        let version = output.ok().filter(|o| o.status.success()).and_then(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .next()
                .map(str::to_string)
        });
        ToolStatus {
            name: name.to_string(),
            available: version.is_some(),
            version,
        }
    }
}

#[async_trait]
impl DownloaderEngine for YtDlpEngine {
    fn name(&self) -> &'static str {
        "yt-dlp + ffmpeg"
    }

    async fn check_tools(&self) -> Vec<ToolStatus> {
        let (a, b, c) = tokio::join!(
            self.tool_version("yt-dlp", &self.tools.yt_dlp, "--version"),
            self.tool_version("ffmpeg", &self.tools.ffmpeg, "-version"),
            self.tool_version("deno", &self.tools.deno, "--version"),
        );
        vec![a, b, c]
    }

    async fn fetch_metadata(&self, url: &str) -> Result<MediaInfo, EngineError> {
        let child = self.spawn_yt_dlp(args::metadata_args(&self.tools, url))?;
        let pid = child.id();
        let output = tokio::time::timeout(METADATA_TIMEOUT, child.wait_with_output()).await;
        self.forget(pid);
        let output = output
            .map_err(|_| {
                EngineError::Message(t!(
                    "Link analysis timed out.",
                    "Bağlantı analizi zaman aşımına uğradı."
                ))
            })?
            .map_err(|source| EngineError::Spawn {
                tool: "yt-dlp",
                source,
            })?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(EngineError::Message(friendly_error(&stderr)));
        }
        let info = metadata::parse_media_info(&String::from_utf8_lossy(&output.stdout), url)
            .map_err(|e| {
                EngineError::Message(t!(
                    "Could not read yt-dlp output: {e}",
                    "yt-dlp çıktısı okunamadı: {e}"
                ))
            })?;
        if info.is_playlist && info.entries.is_empty() {
            return Err(EngineError::Message(t!(
                "No downloadable videos found in this playlist.",
                "Bu listede indirilebilir video bulunamadı."
            )));
        }
        Ok(info)
    }

    async fn download(
        &self,
        job: &DownloadJob,
        output_dir: &Path,
        temp_dir: &Path,
        on_progress: ProgressSink,
        cancel: CancelSignal,
    ) -> Result<PathBuf, EngineError> {
        let args = args::download_args(&self.tools, job, output_dir, temp_dir);
        let tracker = ProgressTracker::new(job.est_video_bytes, job.est_audio_bytes);
        self.transfer(args, tracker, &on_progress, cancel).await
    }

    fn shutdown(&self) {
        for pid in self.running.lock().unwrap().drain() {
            signal_group(pid, false);
        }
    }
}

/// Apps opened from Finder have no Homebrew in PATH, so known folders are checked too.
fn find_aria2c() -> Option<PathBuf> {
    let name = format!("aria2c{}", std::env::consts::EXE_SUFFIX);
    std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .unwrap_or_default()
        .into_iter()
        .chain(["/opt/homebrew/bin", "/usr/local/bin"].map(PathBuf::from))
        .map(|dir| dir.join(&name))
        .find(|p| p.is_file())
}

async fn terminate(child: &mut Child) {
    let Some(pid) = child.id() else { return };
    signal_group(pid, false);
    if tokio::time::timeout(KILL_GRACE, child.wait())
        .await
        .is_err()
    {
        signal_group(pid, true);
        let _ = child.wait().await;
    }
}

#[cfg(unix)]
fn signal_group(pid: u32, force: bool) {
    let signal = if force { libc::SIGKILL } else { libc::SIGTERM };
    // SAFETY: only signals the process group we started ourselves.
    unsafe {
        libc::kill(-(pid as i32), signal);
    }
}

#[cfg(windows)]
fn signal_group(pid: u32, _force: bool) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let _ = std::process::Command::new("taskkill")
        .args(["/T", "/F", "/PID", &pid.to_string()])
        .creation_flags(CREATE_NO_WINDOW)
        .status();
}

/// Turns a raw yt-dlp error into a clear message in the user's language.
pub fn friendly_error(stderr: &str) -> String {
    let raw = stderr
        .lines()
        .rev()
        .find(|l| l.starts_with("ERROR:"))
        .unwrap_or_else(|| stderr.lines().last().unwrap_or(""))
        .trim_start_matches("ERROR:")
        .trim();
    let lower = raw.to_lowercase();
    let has = |needles: &[&str]| needles.iter().any(|n| lower.contains(n));

    if has(&["requested format is not available", "no video formats"]) {
        t!(
            "The chosen quality is not available. Try another quality.",
            "Seçilen kalite mevcut değil. Başka bir kalite deneyin."
        )
    } else if has(&["unsupported url"]) {
        t!("This link is not supported.", "Bu bağlantı desteklenmiyor.")
    } else if has(&["drm"]) {
        t!(
            "This content is DRM-protected and cannot be downloaded.",
            "Bu içerik DRM ile korunuyor, indirilemez."
        )
    } else if has(&["private video", "this video is private"]) {
        t!("This video is private.", "Bu video gizli.")
    } else if has(&[
        "ip address is blocked",
        "not available in your country",
        "geo",
    ]) {
        t!(
            "This content is not available in your location.",
            "Bu içerik bulunduğunuz konumdan erişime kapalı."
        )
    } else if has(&[
        "login",
        "sign in",
        "cookies",
        "empty media response",
        "authentication",
    ]) {
        t!(
            "This content requires signing in. Vana only downloads public content.",
            "Bu içerik oturum açmayı gerektiriyor. Vana yalnızca herkese açık içerikleri indirir."
        )
    } else if has(&[
        "http error 404",
        "video unavailable",
        "is unavailable",
        "has been removed",
        "not found",
    ]) {
        t!(
            "Content not found or removed.",
            "İçerik bulunamadı veya kaldırılmış."
        )
    } else if has(&[
        "unable to download",
        "timed out",
        "name resolution",
        "connection",
    ]) {
        t!(
            "Network error. Check your connection and try again.",
            "Ağ hatası oluştu. Bağlantınızı kontrol edip tekrar deneyin."
        )
    } else if has(&["no space left"]) {
        t!("Not enough disk space.", "Diskte yeterli alan yok.")
    } else if raw.is_empty() {
        t!(
            "yt-dlp stopped unexpectedly.",
            "yt-dlp beklenmedik şekilde durdu."
        )
    } else {
        t!("Download failed: {raw}", "İndirme başarısız: {raw}")
    }
}
