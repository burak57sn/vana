//! Builds yt-dlp command lines. It runs no processes, so it compiles and is tested on every platform.

use std::path::{Path, PathBuf};

use super::model::*;
use super::progress::{PATH_PREFIX, POSTPROCESS_TEMPLATE, PROGRESS_TEMPLATE};

/// Number of fragments downloaded at once for fragmented streams (HLS, DASH).
const CONCURRENT_FRAGMENTS: &str = "16";

/// Tool paths used by every yt-dlp call.
#[derive(Debug, Clone)]
pub struct ToolPaths {
    pub yt_dlp: PathBuf,
    pub ffmpeg: PathBuf,
    pub deno: PathBuf,
    /// Used when installed on the system; not bundled.
    pub aria2c: Option<PathBuf>,
}

pub fn base_args(tools: &ToolPaths) -> Vec<String> {
    vec![
        "--ignore-config".to_string(),
        "--no-update".to_string(),
        "--no-color".to_string(),
        "--encoding".to_string(),
        "utf-8".to_string(),
        "--ffmpeg-location".to_string(),
        tools.ffmpeg.to_string_lossy().into_owned(),
        // YouTube's signature solving needs a JS runtime.
        "--js-runtimes".to_string(),
        format!("deno:{}", tools.deno.to_string_lossy()),
    ]
}

pub fn metadata_args(tools: &ToolPaths, url: &str) -> Vec<String> {
    let mut args = base_args(tools);
    // `--no-playlist` only reduces links like `watch?v=…&list=…` to a single video;
    // a playlist's own link still opens as a playlist.
    args.extend(
        [
            "-J",
            "--no-warnings",
            "--no-playlist",
            "--flat-playlist",
            "--playlist-items",
            "1:1000",
        ]
        .map(String::from),
    );
    args.push("--".to_string());
    args.push(url.to_string());
    args
}

pub fn video_format_selector(height: Option<u32>) -> String {
    let limit = height.map_or(String::new(), |h| format!("[height<=?{h}]"));
    let limited = format!("bv*{limit}+ba/b{limit}");
    if limit.is_empty() {
        return limited;
    }
    // If nothing is under the chosen limit, fall back to the best source.
    format!("{limited}/bv*+ba/b")
}

/// Shared by all downloads: speed, progress output and folders.
fn transfer_args(
    tools: &ToolPaths,
    output_dir: &Path,
    temp_dir: &Path,
    output_template: &str,
) -> Vec<String> {
    let mut args = base_args(tools);
    args.extend(
        [
            "--no-playlist",
            "--newline",
            "--no-quiet",
            "--no-mtime",
            "--concurrent-fragments",
            CONCURRENT_FRAGMENTS,
            "--buffer-size",
            "16K",
            "--progress-template",
        ]
        .map(String::from),
    );
    if let Some(aria2c) = &tools.aria2c {
        args.push("--external-downloader".to_string());
        args.push(aria2c.to_string_lossy().into_owned());
        args.push("--external-downloader-args".to_string());
        args.push("aria2c:-x 16 -s 16 -k 1M".to_string());
    }
    args.push(format!("download:{PROGRESS_TEMPLATE}"));
    args.push("--progress-template".to_string());
    args.push(format!("postprocess:{POSTPROCESS_TEMPLATE}"));
    args.push("--print".to_string());
    args.push(format!("after_move:{PATH_PREFIX}%(filepath)s"));

    // Intermediate files stay in the temp folder; only the final file lands in Downloads.
    args.push("--paths".to_string());
    args.push(format!("home:{}", output_dir.to_string_lossy()));
    args.push("--paths".to_string());
    args.push(format!("temp:{}", temp_dir.to_string_lossy()));
    args.push("--windows-filenames".to_string());
    args.push("--output".to_string());
    args.push(output_template.to_string());
    args
}

pub fn download_args(
    tools: &ToolPaths,
    job: &DownloadJob,
    output_dir: &Path,
    temp_dir: &Path,
) -> Vec<String> {
    // Quality label: another quality of the same video must not count as "already downloaded".
    let template = format!(
        "%(title).150B [{}] [%(id)s].%(ext)s",
        sanitize_label(&job.quality_label)
    );
    let mut args = transfer_args(tools, output_dir, temp_dir, &template);

    match &job.selection {
        Selection::Video {
            container,
            height,
            subtitles,
        } => {
            let ext = container.as_str();
            args.push("-f".to_string());
            args.push(video_format_selector(*height));
            if *container == VideoContainer::Mp4 {
                // Prefer H.264/AAC at the same resolution so the file plays everywhere.
                args.push("-S".to_string());
                args.push("res,fps,vcodec:h264,acodec:aac".to_string());
            }
            args.extend(["--merge-output-format", ext, "--remux-video", ext].map(String::from));
            args.push("--add-metadata".to_string());
            if *subtitles {
                args.extend(
                    [
                        "--write-subs",
                        "--write-auto-subs",
                        "--sub-langs",
                        "tr,tr-orig,en,en-orig,-live_chat",
                        "--embed-subs",
                        // If the subtitle server refuses, still download the video.
                        "--ignore-errors",
                    ]
                    .map(String::from),
                );
            }
        }
        Selection::Audio { format, kbps } => {
            args.extend(["-f", "ba/b", "-x", "--audio-format"].map(String::from));
            args.push(format.as_str().to_string());
            if let Some(kbps) = kbps.filter(|_| *format != AudioFormat::Wav) {
                args.push("--audio-quality".to_string());
                args.push(format!("{kbps}K"));
            }
            args.push("--add-metadata".to_string());
            if *format != AudioFormat::Wav {
                // WAV cannot carry cover art.
                args.push("--embed-thumbnail".to_string());
            }
        }
    }

    args.push("--".to_string());
    args.push(job.url.clone());
    args
}

fn sanitize_label(label: &str) -> String {
    label
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ')
        .collect::<String>()
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tools() -> ToolPaths {
        ToolPaths {
            yt_dlp: "/b/yt-dlp".into(),
            ffmpeg: "/b/ffmpeg".into(),
            deno: "/b/deno".into(),
            aria2c: None,
        }
    }

    fn job(selection: Selection, label: &str) -> DownloadJob {
        DownloadJob {
            url: "https://example.com/v".into(),
            selection,
            quality_label: label.into(),
            est_video_bytes: None,
            est_audio_bytes: None,
        }
    }

    fn has_pair(args: &[String], a: &str, b: &str) -> bool {
        args.windows(2).any(|w| w[0] == a && w[1] == b)
    }

    #[test]
    fn video_mode_merges_into_real_mp4() {
        let args = download_args(
            &tools(),
            &job(
                Selection::Video {
                    container: VideoContainer::Mp4,
                    height: Some(1080),
                    subtitles: false,
                },
                "1080p",
            ),
            Path::new("/d"),
            Path::new("/t"),
        );
        assert!(has_pair(&args, "--merge-output-format", "mp4"));
        assert!(has_pair(&args, "--remux-video", "mp4"));
        let selector = &args[args.iter().position(|a| a == "-f").unwrap() + 1];
        assert_eq!(selector, "bv*[height<=?1080]+ba/b[height<=?1080]/bv*+ba/b");
    }

    fn mp4_job() -> DownloadJob {
        job(
            Selection::Video {
                container: VideoContainer::Mp4,
                height: Some(1080),
                subtitles: false,
            },
            "1080p",
        )
    }

    #[test]
    fn download_uses_parallel_fragments_and_never_reencodes_video() {
        let args = download_args(&tools(), &mp4_job(), Path::new("/d"), Path::new("/t"));
        assert!(has_pair(&args, "--concurrent-fragments", "16"));
        assert!(has_pair(&args, "--buffer-size", "16K"));
        assert!(!args.contains(&"--external-downloader".to_string()));
        // `--recode-video` re-encodes; `--remux-video` only copies streams.
        assert!(!args.contains(&"--recode-video".to_string()));
    }

    #[test]
    fn aria2c_is_used_only_when_found() {
        let mut with_aria = tools();
        with_aria.aria2c = Some("/opt/homebrew/bin/aria2c".into());
        let args = download_args(&with_aria, &mp4_job(), Path::new("/d"), Path::new("/t"));
        assert!(has_pair(
            &args,
            "--external-downloader",
            "/opt/homebrew/bin/aria2c"
        ));
        assert!(has_pair(
            &args,
            "--external-downloader-args",
            "aria2c:-x 16 -s 16 -k 1M"
        ));
    }

    #[test]
    fn metadata_skips_warnings_and_parent_playlist() {
        let args = metadata_args(&tools(), "https://example.com/v");
        for flag in ["--no-warnings", "--no-playlist", "--flat-playlist"] {
            assert!(args.contains(&flag.to_string()), "{flag}");
        }
    }

    #[test]
    fn mp3_mode_converts_and_embeds_cover() {
        let args = download_args(
            &tools(),
            &job(
                Selection::Audio {
                    format: AudioFormat::Mp3,
                    kbps: Some(320),
                },
                "320kbps",
            ),
            Path::new("/d"),
            Path::new("/t"),
        );
        assert!(args.contains(&"-x".to_string()));
        assert!(has_pair(&args, "--audio-format", "mp3"));
        assert!(has_pair(&args, "--audio-quality", "320K"));
        assert!(args.contains(&"--embed-thumbnail".to_string()));
        assert!(args.contains(&"--add-metadata".to_string()));
    }

    #[test]
    fn wav_has_no_bitrate_or_cover() {
        let args = download_args(
            &tools(),
            &job(
                Selection::Audio {
                    format: AudioFormat::Wav,
                    kbps: Some(320),
                },
                "WAV",
            ),
            Path::new("/d"),
            Path::new("/t"),
        );
        assert!(!args.contains(&"--audio-quality".to_string()));
        assert!(!args.contains(&"--embed-thumbnail".to_string()));
    }

    #[test]
    fn output_label_keeps_turkish_letters_and_drops_template_syntax() {
        assert_eq!(sanitize_label("Kayıpsız"), "Kayıpsız");
        assert_eq!(sanitize_label("1080p%(id)s/../"), "1080pids");
    }

    #[test]
    fn url_cannot_be_read_as_an_option() {
        let mut j = job(
            Selection::Audio {
                format: AudioFormat::Mp3,
                kbps: None,
            },
            "x",
        );
        j.url = "--exec=evil".into();
        let args = download_args(&tools(), &j, Path::new("/d"), Path::new("/t"));
        assert_eq!(&args[args.len() - 2..], ["--", "--exec=evil"]);
    }
}
