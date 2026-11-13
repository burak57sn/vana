//! Android engine skeleton.
//!
//! The desktop sidecars do not run on Android. A native integration (for example a
//! Kotlin Tauri plugin using youtubedl-android) implements [`AndroidBackend`] and is
//! plugged in with `AndroidEngine::with_backend`. The queue, commands and UI stay the same.
//!
//! Steps:
//! 1. Generate `src-tauri/gen/android` with `npm run android:init`.
//! 2. Write the Kotlin plugin and register it in `tauri::plugin::Builder` with
//!    `api.register_android_plugin("io.github.burak57sn.vana", "VanaEnginePlugin")`.
//! 3. Route the `AndroidBackend` methods to `run_mobile_plugin` calls.
//! 4. Return `AndroidEngine::with_backend(...)` from `engine::create_engine`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;

use super::*;

const NOT_READY: &str = "Android download engine";

/// Operations the native Android side must provide.
#[async_trait]
pub trait AndroidBackend: Send + Sync {
    async fn fetch_metadata_json(&self, url: &str) -> Result<String, EngineError>;
    async fn download(
        &self,
        job: &DownloadJob,
        output_dir: &Path,
        on_progress: ProgressSink,
        cancel: CancelSignal,
    ) -> Result<PathBuf, EngineError>;
}

#[derive(Default)]
pub struct AndroidEngine {
    backend: Option<Arc<dyn AndroidBackend>>,
}

impl AndroidEngine {
    pub fn with_backend(backend: Arc<dyn AndroidBackend>) -> Self {
        Self {
            backend: Some(backend),
        }
    }

    fn backend(&self) -> Result<&Arc<dyn AndroidBackend>, EngineError> {
        self.backend
            .as_ref()
            .ok_or(EngineError::Unsupported(NOT_READY))
    }
}

#[async_trait]
impl DownloaderEngine for AndroidEngine {
    fn name(&self) -> &'static str {
        "android"
    }

    async fn check_tools(&self) -> Vec<ToolStatus> {
        vec![ToolStatus {
            name: NOT_READY.to_string(),
            available: self.backend.is_some(),
            version: None,
        }]
    }

    async fn fetch_metadata(&self, url: &str) -> Result<MediaInfo, EngineError> {
        let json = self.backend()?.fetch_metadata_json(url).await?;
        metadata::parse_media_info(&json, url).map_err(|e| EngineError::Message(e.to_string()))
    }

    async fn download(
        &self,
        job: &DownloadJob,
        output_dir: &Path,
        _temp_dir: &Path,
        on_progress: ProgressSink,
        cancel: CancelSignal,
    ) -> Result<PathBuf, EngineError> {
        self.backend()?
            .download(job, output_dir, on_progress, cancel)
            .await
    }
}
