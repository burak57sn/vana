//! Multi-download queue. State lives here; every change is emitted to the UI as an event.

use crate::t;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::path::BaseDirectory;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::watch;

use crate::engine::{DownloadJob, DownloaderEngine, EngineError, ProgressUpdate, Stage};

pub const EVENT_UPDATE: &str = "download:update";
pub const EVENT_REMOVED: &str = "download:removed";

const MAX_PARALLEL: usize = 3;
const EMIT_INTERVAL: Duration = Duration::from_millis(200);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    Queued,
    Active,
    Completed,
    Failed,
    Canceled,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadRequest {
    pub job: DownloadJob,
    pub title: String,
    pub thumbnail: Option<String>,
    pub platform: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadItem {
    pub id: String,
    pub title: String,
    pub thumbnail: Option<String>,
    pub platform: String,
    /// `MP4`, `MP3`…
    pub tag: String,
    /// `1080p`, `320kbps`…
    pub quality: String,
    pub status: Status,
    pub stage: Stage,
    pub percent: f64,
    pub speed_bps: Option<f64>,
    pub eta_secs: Option<u64>,
    pub error: Option<String>,
    pub file_path: Option<String>,
    #[serde(skip)]
    job: Option<DownloadJob>,
}

#[derive(Default)]
struct State {
    /// In the order they were added.
    items: Vec<DownloadItem>,
    running: HashMap<String, watch::Sender<bool>>,
}

#[derive(Clone)]
pub struct DownloadManager {
    app: AppHandle,
    engine: Arc<dyn DownloaderEngine>,
    state: Arc<Mutex<State>>,
}

impl DownloadManager {
    pub fn new(app: AppHandle, engine: Arc<dyn DownloaderEngine>) -> Self {
        Self {
            app,
            engine,
            state: Arc::default(),
        }
    }

    pub fn engine(&self) -> &Arc<dyn DownloaderEngine> {
        &self.engine
    }

    pub fn list(&self) -> Vec<DownloadItem> {
        self.state.lock().unwrap().items.clone()
    }

    pub fn enqueue(&self, request: DownloadRequest) -> DownloadItem {
        let item = DownloadItem {
            id: uuid::Uuid::new_v4().to_string(),
            title: request.title,
            thumbnail: request.thumbnail,
            platform: request.platform,
            tag: request.job.selection.extension().to_uppercase(),
            quality: request.job.quality_label.clone(),
            status: Status::Queued,
            stage: Stage::Downloading,
            percent: 0.0,
            speed_bps: None,
            eta_secs: None,
            error: None,
            file_path: None,
            job: Some(request.job),
        };
        self.state.lock().unwrap().items.push(item.clone());
        self.emit(&item);
        self.pump();
        item
    }

    pub fn cancel(&self, id: &str) {
        let mut state = self.state.lock().unwrap();
        if let Some(sender) = state.running.get(id) {
            let _ = sender.send(true);
            return;
        }
        let queued = state
            .items
            .iter_mut()
            .find(|i| i.id == id && i.status == Status::Queued);
        if let Some(item) = queued {
            item.status = Status::Canceled;
            let snapshot = item.clone();
            drop(state);
            self.emit(&snapshot);
        }
    }

    pub fn retry(&self, id: &str) {
        let snapshot = {
            let mut state = self.state.lock().unwrap();
            let Some(item) = state
                .items
                .iter_mut()
                .find(|i| i.id == id && matches!(i.status, Status::Failed | Status::Canceled))
            else {
                return;
            };
            item.status = Status::Queued;
            item.stage = Stage::Downloading;
            item.percent = 0.0;
            item.error = None;
            item.clone()
        };
        self.emit(&snapshot);
        self.pump();
    }

    pub fn remove(&self, id: &str) {
        {
            let mut state = self.state.lock().unwrap();
            if let Some(sender) = state.running.remove(id) {
                let _ = sender.send(true);
            }
            state.items.retain(|i| i.id != id);
        }
        self.clean_temp(id);
        let _ = self.app.emit(EVENT_REMOVED, id);
        self.update_tray();
    }

    #[cfg(desktop)]
    pub fn completed_path(&self, id: &str) -> Option<PathBuf> {
        let state = self.state.lock().unwrap();
        let item = state.items.iter().find(|i| i.id == id)?;
        item.file_path.as_ref().map(PathBuf::from)
    }

    pub fn download_dir(&self) -> Result<PathBuf, EngineError> {
        self.app
            .path()
            .resolve("", BaseDirectory::Download)
            .map_err(|e| {
                EngineError::Message(t!(
                    "Downloads folder not found: {e}",
                    "İndirilenler klasörü bulunamadı: {e}"
                ))
            })
    }

    fn temp_dir(&self, id: &str) -> Option<PathBuf> {
        Some(self.app.path().app_cache_dir().ok()?.join("jobs").join(id))
    }

    fn clean_temp(&self, id: &str) {
        if let Some(dir) = self.temp_dir(id) {
            let _ = std::fs::remove_dir_all(dir);
        }
    }

    /// Starts queued jobs while there are free slots.
    fn pump(&self) {
        let mut started = Vec::new();
        {
            let mut guard = self.state.lock().unwrap();
            let state = &mut *guard;
            for item in state.items.iter_mut() {
                if state.running.len() >= MAX_PARALLEL {
                    break;
                }
                if item.status == Status::Queued {
                    item.status = Status::Active;
                    let (tx, rx) = watch::channel(false);
                    state.running.insert(item.id.clone(), tx);
                    started.push((item.clone(), rx));
                }
            }
        }
        for (item, cancel) in started {
            self.emit(&item);
            let manager = self.clone();
            tauri::async_runtime::spawn(async move { manager.run(item, cancel).await });
        }
    }

    async fn run(self, item: DownloadItem, cancel: watch::Receiver<bool>) {
        let id = item.id.clone();
        let job = item.job.clone().expect("queued item must have a job");

        let last_emit = Mutex::new(Instant::now() - EMIT_INTERVAL);
        let sink = {
            let manager = self.clone();
            let id = id.clone();
            Arc::new(move |update: ProgressUpdate| {
                let mut last = last_emit.lock().unwrap();
                let stage_changed = manager.apply_progress(&id, &update);
                if stage_changed || last.elapsed() >= EMIT_INTERVAL {
                    *last = Instant::now();
                    manager.emit_by_id(&id);
                }
            })
        };

        let result = async {
            let output_dir = self.download_dir()?;
            let temp_dir = self.temp_dir(&id).ok_or_else(|| {
                EngineError::Message(t!(
                    "Temporary folder not found.",
                    "Geçici klasör bulunamadı."
                ))
            })?;
            for dir in [&output_dir, &temp_dir] {
                std::fs::create_dir_all(dir).map_err(|e| {
                    EngineError::Message(t!(
                        "Could not create folder: {e}",
                        "Klasör oluşturulamadı: {e}"
                    ))
                })?;
            }
            self.engine
                .download(&job, &output_dir, &temp_dir, sink, cancel)
                .await
        }
        .await;

        let removed = {
            let mut guard = self.state.lock().unwrap();
            guard.running.remove(&id);
            match guard.items.iter_mut().find(|i| i.id == id) {
                Some(item) => {
                    item.speed_bps = None;
                    item.eta_secs = None;
                    match &result {
                        Ok(path) => {
                            item.status = Status::Completed;
                            item.stage = Stage::Finishing;
                            item.percent = 100.0;
                            item.file_path = Some(path.to_string_lossy().into_owned());
                        }
                        Err(EngineError::Canceled) => item.status = Status::Canceled,
                        Err(e) => {
                            item.status = Status::Failed;
                            item.error = Some(e.to_string());
                        }
                    }
                    false
                }
                None => true,
            }
        };

        // Partial files of a failed job are kept so "Retry" can resume where it stopped.
        if removed || !matches!(result, Err(ref e) if !matches!(e, EngineError::Canceled)) {
            self.clean_temp(&id);
        }
        self.emit_by_id(&id);
        self.pump();
    }

    /// Returns `true` if the stage changed.
    fn apply_progress(&self, id: &str, update: &ProgressUpdate) -> bool {
        let mut state = self.state.lock().unwrap();
        let Some(item) = state.items.iter_mut().find(|i| i.id == id) else {
            return false;
        };
        let changed = item.stage != update.stage;
        item.percent = update.percent;
        item.speed_bps = update.speed_bps;
        item.eta_secs = update.eta_secs;
        item.stage = update.stage;
        changed
    }

    fn emit_by_id(&self, id: &str) {
        let snapshot = {
            let state = self.state.lock().unwrap();
            state.items.iter().find(|i| i.id == id).cloned()
        };
        match snapshot {
            Some(item) => self.emit(&item),
            None => self.update_tray(),
        }
    }

    fn emit(&self, item: &DownloadItem) {
        let _ = self.app.emit(EVENT_UPDATE, item);
        self.update_tray();
    }

    /// Turns the menu bar icon according to the average progress of active downloads.
    fn update_tray(&self) {
        #[cfg(desktop)]
        {
            let (count, percent) = {
                let state = self.state.lock().unwrap();
                let active: Vec<f64> = state
                    .items
                    .iter()
                    .filter(|i| i.status == Status::Active)
                    .map(|i| i.percent)
                    .collect();
                let average = active.iter().sum::<f64>() / active.len().max(1) as f64;
                (active.len(), average)
            };
            crate::tray::update(&self.app, count, percent);
        }
    }
}
