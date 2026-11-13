//! End-to-end tests against the real network and sidecars:
//! `cargo test --test e2e -- --ignored --nocapture --test-threads=1`
//! (run `cargo build` first so the sidecars are copied into target/debug)
#![cfg(desktop)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::watch;
use vana_lib::engine::desktop::YtDlpEngine;
use vana_lib::engine::*;

const SHORT_VIDEO: &str = "https://www.youtube.com/watch?v=jNQXAC9IVRw";
const LONG_VIDEO: &str = "https://www.youtube.com/watch?v=aqz-KE-bpKQ";
const TIKTOK: &str = "https://www.tiktok.com/@charlidamelio/video/7137423965982592302";

/// `VANA_BIN_DIR=…/Vana.app/Contents/MacOS` tests the tools inside a packaged app.
fn bin_dir() -> PathBuf {
    std::env::var_os("VANA_BIN_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("target/debug"))
}

fn engine() -> YtDlpEngine {
    YtDlpEngine::from_dir(&bin_dir())
}

fn workdir(name: &str) -> (PathBuf, PathBuf) {
    let root = std::env::temp_dir().join(format!("vana-e2e-{}-{name}", std::process::id()));
    let (out, tmp) = (root.join("out"), root.join("tmp"));
    std::fs::create_dir_all(&out).unwrap();
    std::fs::create_dir_all(&tmp).unwrap();
    (out, tmp)
}

fn probe(path: &Path) -> serde_json::Value {
    let output = Command::new(bin_dir().join("ffprobe"))
        .args([
            "-v",
            "error",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(path)
        .output()
        .unwrap();
    serde_json::from_slice(&output.stdout).unwrap()
}

fn job(url: &str, selection: Selection, label: &str) -> DownloadJob {
    DownloadJob {
        url: url.into(),
        selection,
        quality_label: label.into(),
        est_video_bytes: None,
        est_audio_bytes: None,
    }
}

async fn run(job: &DownloadJob, name: &str) -> (PathBuf, Vec<ProgressUpdate>) {
    let (out, tmp) = workdir(name);
    let updates = Arc::new(Mutex::new(Vec::new()));
    let sink_updates = updates.clone();
    let (_tx, rx) = watch::channel(false);
    let path = engine()
        .download(
            job,
            &out,
            &tmp,
            Arc::new(move |u| sink_updates.lock().unwrap().push(u)),
            rx,
        )
        .await
        .unwrap();
    let collected = updates.lock().unwrap().clone();
    (path, collected)
}

#[tokio::test]
#[ignore]
async fn tools_are_available() {
    let tools = engine().check_tools().await;
    println!("{tools:#?}");
    assert!(tools.iter().all(|t| t.available));
}

#[tokio::test]
#[ignore]
async fn youtube_metadata_and_estimate_is_close() {
    let info = engine().fetch_metadata(LONG_VIDEO).await.unwrap();
    assert_eq!(info.platform, "YouTube");
    let labels: Vec<_> = info
        .video_options
        .iter()
        .map(|o| o.short_label.clone())
        .collect();
    println!("{labels:?} source audio: {:?}", info.audio_source_kbps);
    assert!(labels.contains(&"4K".to_string()) && labels.contains(&"1080p".to_string()));
    assert_eq!(info.audio_options.len(), 4);
    for o in &info.video_options {
        println!(
            "{:>14} h={} mp4={:?} mkv={:?}",
            o.label, o.height, o.size_mp4, o.size_mkv
        );
    }
}

#[tokio::test]
#[ignore]
async fn video_mode_produces_real_mp4_with_both_streams() {
    let selection = Selection::Video {
        container: VideoContainer::Mp4,
        height: Some(240),
        subtitles: false,
    };
    let (path, updates) = run(&job(SHORT_VIDEO, selection, "240p"), "mp4").await;
    println!("{path:?} — {} progress lines", updates.len());
    assert_eq!(path.extension().unwrap(), "mp4");
    let info = probe(&path);
    assert!(info["format"]["format_name"]
        .as_str()
        .unwrap()
        .contains("mp4"));
    let kinds: Vec<_> = info["streams"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["codec_type"].as_str().unwrap())
        .collect();
    assert!(
        kinds.contains(&"video") && kinds.contains(&"audio"),
        "{kinds:?}"
    );
    assert!(!updates.is_empty());
    assert!(updates.windows(2).all(|w| w[0].percent <= w[1].percent));
}

#[tokio::test]
#[ignore]
async fn audio_mode_produces_mp3_with_cover_and_title() {
    let selection = Selection::Audio {
        format: AudioFormat::Mp3,
        kbps: Some(320),
    };
    let (path, _) = run(&job(SHORT_VIDEO, selection, "320kbps"), "mp3").await;
    println!("{path:?}");
    let info = probe(&path);
    assert_eq!(info["format"]["format_name"], "mp3");
    // Cover art inflates the container bitrate, so check the audio stream.
    let audio = info["streams"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["codec_type"] == "audio")
        .unwrap();
    let bitrate: u64 = audio["bit_rate"].as_str().unwrap().parse().unwrap();
    assert!((310_000..=330_000).contains(&bitrate), "{bitrate}");
    assert!(info["format"]["tags"]["title"].as_str().is_some());
    let streams = info["streams"].as_array().unwrap();
    assert!(
        streams.iter().any(|s| s["codec_type"] == "video"),
        "no cover"
    );
}

#[tokio::test]
#[ignore]
async fn m4a_and_wav_formats() {
    let m4a = Selection::Audio {
        format: AudioFormat::M4a,
        kbps: Some(128),
    };
    let (path, _) = run(&job(SHORT_VIDEO, m4a, "128kbps"), "m4a").await;
    assert_eq!(path.extension().unwrap(), "m4a");
    let wav = Selection::Audio {
        format: AudioFormat::Wav,
        kbps: None,
    };
    let (path, _) = run(&job(SHORT_VIDEO, wav, "WAV"), "wav").await;
    assert_eq!(path.extension().unwrap(), "wav");
    assert_eq!(probe(&path)["format"]["format_name"], "wav");
}

#[tokio::test]
#[ignore]
async fn mkv_container_with_subtitles_option() {
    let selection = Selection::Video {
        container: VideoContainer::Mkv,
        height: Some(240),
        subtitles: true,
    };
    let (path, _) = run(&job(SHORT_VIDEO, selection, "240p"), "mkv").await;
    assert_eq!(path.extension().unwrap(), "mkv");
    println!(
        "{}",
        probe(&path)["streams"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["codec_type"].to_string())
            .collect::<Vec<_>>()
            .join(",")
    );
}

#[tokio::test]
#[ignore]
async fn tiktok_video_is_downloaded() {
    let info = engine().fetch_metadata(TIKTOK).await.unwrap();
    let selection = Selection::Video {
        container: VideoContainer::Mp4,
        height: Some(info.video_options[0].height),
        subtitles: false,
    };
    let (path, _) = run(
        &job(TIKTOK, selection, &info.video_options[0].short_label),
        "tiktok",
    )
    .await;
    println!("{path:?} {:?}", info.video_options[0]);
    assert!(probe(&path)["format"]["format_name"]
        .as_str()
        .unwrap()
        .contains("mp4"));
}

#[tokio::test]
#[ignore]
async fn cancel_stops_the_whole_process_tree() {
    let (out, tmp) = workdir("cancel");
    let (tx, rx) = watch::channel(false);
    let selection = Selection::Video {
        container: VideoContainer::Mp4,
        height: Some(1080),
        subtitles: false,
    };
    let job = job(LONG_VIDEO, selection, "1080p");
    let engine = engine();
    let task = engine.download(&job, &out, &tmp, Arc::new(|_| {}), rx);
    tokio::pin!(task);
    tokio::select! {
        _ = &mut task => panic!("finished before cancel"),
        _ = tokio::time::sleep(Duration::from_secs(20)) => {}
    }
    tx.send(true).unwrap();
    assert!(matches!(task.await, Err(EngineError::Canceled)));
    tokio::time::sleep(Duration::from_secs(2)).await;
    let left = Command::new("pgrep")
        .args(["-f", &tmp.to_string_lossy()])
        .output()
        .unwrap();
    assert!(
        left.stdout.is_empty(),
        "leftover process: {}",
        String::from_utf8_lossy(&left.stdout)
    );
}

#[tokio::test]
#[ignore]
async fn split_streams_report_merge_stage() {
    let selection = Selection::Video {
        container: VideoContainer::Mp4,
        height: Some(144),
        subtitles: false,
    };
    let (path, updates) = run(&job(LONG_VIDEO, selection, "144p"), "merge").await;
    println!("{path:?}");
    assert!(updates.iter().any(|u| u.stage == Stage::Merging));
    let kinds = probe(&path)["streams"].to_string();
    assert!(kinds.contains("\"video\"") && kinds.contains("\"audio\""));
}

#[tokio::test]
#[ignore]
async fn playlist_returns_every_entry() {
    let url = "https://www.youtube.com/@BlenderStudio/videos";
    let info = engine().fetch_metadata(url).await.unwrap();
    println!(
        "{} — {} videos, first: {:?}",
        info.title,
        info.entries.len(),
        info.entries.first()
    );
    assert!(info.is_playlist && !info.entries.is_empty());
    assert!(info.entries.iter().all(|e| e.url.starts_with("http")));
}

#[tokio::test]
#[ignore]
async fn empty_playlist_gives_a_clear_error() {
    let url = "https://www.youtube.com/playlist?list=PL4lCao7KL_QFVb7Iudeipvc2BCavECqzc";
    let error = engine().fetch_metadata(url).await.unwrap_err();
    assert!(
        error.to_string().contains("No downloadable videos"),
        "{error}"
    );
}

#[tokio::test]
#[ignore]
async fn watch_url_with_list_is_single_video_and_offers_playlist() {
    let url = "https://www.youtube.com/watch?v=jNQXAC9IVRw&list=PL4lCao7KL_QFVb7Iudeipvc2BCavECqzc";
    let info = engine().fetch_metadata(url).await.unwrap();
    assert!(!info.is_playlist);
    assert!(info.parent_playlist_url.is_some());
}

#[tokio::test]
#[ignore]
async fn x_video_metadata() {
    let info = engine()
        .fetch_metadata("https://x.com/SpaceX/status/1732824684683784516")
        .await
        .unwrap();
    assert_eq!(info.platform, "X");
    let labels: Vec<_> = info.video_options.iter().map(|o| o.label.clone()).collect();
    println!("{labels:?}");
    assert!(!labels.is_empty());
}
