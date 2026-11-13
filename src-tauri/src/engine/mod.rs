//! Download engine abstraction.
//!
//! Commands and the queue only know the [`DownloaderEngine`] trait. On desktop
//! `desktop::YtDlpEngine` runs the sidecar binaries; on Android
//! `mobile::AndroidEngine` is where a native integration plugs in.

pub mod args;
pub mod metadata;
pub mod model;
pub mod progress;

#[cfg(desktop)]
pub mod desktop;
#[cfg(mobile)]
pub mod mobile;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::watch;

pub use model::*;

/// When `true`, the running download is stopped.
pub type CancelSignal = watch::Receiver<bool>;
pub type ProgressSink = Arc<dyn Fn(ProgressUpdate) + Send + Sync>;

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("{0}")]
    Message(String),
    #[error("{}", crate::t!("Download canceled.", "İndirme iptal edildi."))]
    Canceled,
    #[error("{}", unsupported(.0))]
    Unsupported(&'static str),
    #[error("{}", crate::t!("Could not run {tool}: {source}", "{tool} çalıştırılamadı: {source}"))]
    Spawn {
        tool: &'static str,
        source: std::io::Error,
    },
}

fn unsupported(feature: &str) -> String {
    crate::t!(
        "This feature is not supported on this platform yet: {feature}",
        "Bu özellik henüz bu platformda desteklenmiyor: {feature}"
    )
}

#[async_trait]
pub trait DownloaderEngine: Send + Sync {
    fn name(&self) -> &'static str;

    /// Reports the status of the tools the engine needs.
    async fn check_tools(&self) -> Vec<ToolStatus>;

    async fn fetch_metadata(&self, url: &str) -> Result<MediaInfo, EngineError>;

    /// Completes the download for `job` and returns the final file path.
    async fn download(
        &self,
        job: &DownloadJob,
        output_dir: &Path,
        temp_dir: &Path,
        on_progress: ProgressSink,
        cancel: CancelSignal,
    ) -> Result<PathBuf, EngineError>;

    /// Terminates all running child processes when the app exits.
    fn shutdown(&self) {}
}

#[cfg(desktop)]
pub fn create_engine() -> Result<Arc<dyn DownloaderEngine>, EngineError> {
    Ok(Arc::new(desktop::YtDlpEngine::from_sidecars()?))
}

#[cfg(mobile)]
pub fn create_engine() -> Result<Arc<dyn DownloaderEngine>, EngineError> {
    Ok(Arc::new(mobile::AndroidEngine::default()))
}
