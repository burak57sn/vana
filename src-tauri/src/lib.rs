pub mod engine;
pub mod i18n;
mod queue;
#[cfg(desktop)]
mod tray;

use serde::Serialize;
#[cfg(desktop)]
use tauri::AppHandle;
use tauri::{Manager, State};
#[cfg(desktop)]
use tauri_plugin_opener::OpenerExt;

use engine::{MediaInfo, ToolStatus};
use queue::{DownloadItem, DownloadManager, DownloadRequest};

fn validate_url(url: &str) -> Result<String, String> {
    let trimmed = url.trim();
    match url::Url::parse(trimmed) {
        Ok(parsed) if matches!(parsed.scheme(), "http" | "https") => Ok(trimmed.to_string()),
        _ => Err(t!(
            "Enter a valid link (it must start with https://).",
            "Geçerli bir bağlantı girin (https:// ile başlamalı)."
        )),
    }
}

#[tauri::command]
async fn fetch_metadata(
    manager: State<'_, DownloadManager>,
    url: String,
) -> Result<MediaInfo, String> {
    let url = validate_url(&url)?;
    manager
        .engine()
        .fetch_metadata(&url)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn start_download(
    manager: State<'_, DownloadManager>,
    mut request: DownloadRequest,
) -> Result<DownloadItem, String> {
    request.job.url = validate_url(&request.job.url)?;
    Ok(manager.enqueue(request))
}

#[tauri::command]
fn list_downloads(manager: State<'_, DownloadManager>) -> Vec<DownloadItem> {
    manager.list()
}

#[tauri::command]
fn cancel_download(manager: State<'_, DownloadManager>, id: String) {
    manager.cancel(&id);
}

#[tauri::command]
fn retry_download(manager: State<'_, DownloadManager>, id: String) {
    manager.retry(&id);
}

#[tauri::command]
fn remove_download(manager: State<'_, DownloadManager>, id: String) {
    manager.remove(&id);
}

/// Takes an id instead of a path: the UI can only open files Vana downloaded.
#[cfg(desktop)]
#[tauri::command]
fn open_download(
    app: AppHandle,
    manager: State<'_, DownloadManager>,
    id: String,
) -> Result<(), String> {
    let path = manager
        .completed_path(&id)
        .ok_or_else(|| t!("File not found.", "Dosya bulunamadı."))?;
    app.opener()
        .open_path(path.to_string_lossy(), None::<&str>)
        .map_err(|e| t!("Could not open the file: {e}", "Dosya açılamadı: {e}"))
}

#[cfg(desktop)]
#[tauri::command]
fn reveal_download(
    app: AppHandle,
    manager: State<'_, DownloadManager>,
    id: String,
) -> Result<(), String> {
    let path = manager
        .completed_path(&id)
        .ok_or_else(|| t!("File not found.", "Dosya bulunamadı."))?;
    app.opener()
        .reveal_item_in_dir(path)
        .map_err(|e| t!("Could not open the folder: {e}", "Klasör açılamadı: {e}"))
}

#[cfg(mobile)]
#[tauri::command]
fn open_download(_id: String) -> Result<(), String> {
    Err(t!(
        "Opening files is not wired up on Android yet.",
        "Dosya açma Android'de henüz bağlanmadı."
    ))
}

#[cfg(mobile)]
#[tauri::command]
fn reveal_download(_id: String) -> Result<(), String> {
    Err(t!(
        "Showing in folder is not supported on Android.",
        "Klasörde gösterme Android'de desteklenmiyor."
    ))
}

/// Switches the language of backend messages and the tray menu (`en` or `tr`).
#[tauri::command]
fn set_language(app: tauri::AppHandle, language: String) {
    i18n::set_turkish(language == "tr");
    #[cfg(desktop)]
    tray::relabel(&app);
    #[cfg(mobile)]
    let _ = app;
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppEnvironment {
    os: &'static str,
    is_mobile: bool,
    engine: &'static str,
    download_dir: Option<String>,
    tools: Vec<ToolStatus>,
}

#[tauri::command]
async fn app_environment(manager: State<'_, DownloadManager>) -> Result<AppEnvironment, String> {
    Ok(AppEnvironment {
        os: std::env::consts::OS,
        is_mobile: cfg!(mobile),
        engine: manager.engine().name(),
        download_dir: manager
            .download_dir()
            .ok()
            .map(|p| p.to_string_lossy().into_owned()),
        tools: manager.engine().check_tools().await,
    })
}

#[cfg(desktop)]
pub(crate) fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let engine = engine::create_engine().expect("failed to start the download engine");
    let shutdown_engine = engine.clone();

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(move |app| {
            app.manage(DownloadManager::new(app.handle().clone(), engine));
            #[cfg(desktop)]
            tray::init(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            fetch_metadata,
            start_download,
            list_downloads,
            cancel_download,
            retry_download,
            remove_download,
            open_download,
            reveal_download,
            app_environment,
            set_language,
        ]);

    // Closing the window does not stop downloads; the app keeps running in the tray.
    #[cfg(desktop)]
    let builder = builder.on_window_event(|window, event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = window.hide();
        }
    });

    builder
        .build(tauri::generate_context!())
        .expect("failed to start Vana")
        .run(move |_app, event| match event {
            tauri::RunEvent::Exit => shutdown_engine.shutdown(),
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen { .. } => show_main_window(_app),
            _ => {}
        });
}
