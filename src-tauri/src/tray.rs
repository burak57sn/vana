//! Menu bar / system tray icon. The valve turns step by step as downloads progress.

use std::sync::Mutex;

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{TrayIcon, TrayIconBuilder};
use tauri::{AppHandle, Manager};

use crate::t;

const FRAME_COUNT: usize = 12;
/// Frames stepped through from 0% to 100%. For the three-spoke valve 12 frames are 120°,
/// so the valve turns 1⅓ times over a full download.
const STEPS_PER_DOWNLOAD: f64 = 48.0;

macro_rules! frames {
    ($dir:literal) => {
        [
            include_bytes!(concat!("../icons/tray/", $dir, "-00.png")).as_slice(),
            include_bytes!(concat!("../icons/tray/", $dir, "-01.png")).as_slice(),
            include_bytes!(concat!("../icons/tray/", $dir, "-02.png")).as_slice(),
            include_bytes!(concat!("../icons/tray/", $dir, "-03.png")).as_slice(),
            include_bytes!(concat!("../icons/tray/", $dir, "-04.png")).as_slice(),
            include_bytes!(concat!("../icons/tray/", $dir, "-05.png")).as_slice(),
            include_bytes!(concat!("../icons/tray/", $dir, "-06.png")).as_slice(),
            include_bytes!(concat!("../icons/tray/", $dir, "-07.png")).as_slice(),
            include_bytes!(concat!("../icons/tray/", $dir, "-08.png")).as_slice(),
            include_bytes!(concat!("../icons/tray/", $dir, "-09.png")).as_slice(),
            include_bytes!(concat!("../icons/tray/", $dir, "-10.png")).as_slice(),
            include_bytes!(concat!("../icons/tray/", $dir, "-11.png")).as_slice(),
        ]
    };
}

// The macOS menu bar wants a monochrome "template" icon and adapts it to light/dark itself.
#[cfg(target_os = "macos")]
const FRAMES: [&[u8]; FRAME_COUNT] = frames!("mono");
#[cfg(not(target_os = "macos"))]
const FRAMES: [&[u8]; FRAME_COUNT] = frames!("color");

struct TrayState {
    icon: TrayIcon,
    status_item: MenuItem<tauri::Wry>,
    show_item: MenuItem<tauri::Wry>,
    quit_item: MenuItem<tauri::Wry>,
    frame: Mutex<usize>,
    status: Mutex<(usize, u32)>,
}

pub fn frame_for(percent: f64) -> usize {
    let step = (percent.clamp(0.0, 100.0) / 100.0 * STEPS_PER_DOWNLOAD).round() as usize;
    step % FRAME_COUNT
}

pub fn init(app: &AppHandle) -> tauri::Result<()> {
    let status_item = MenuItem::with_id(app, "status", status_text(0, 0), false, None::<&str>)?;
    let show = MenuItem::with_id(
        app,
        "show",
        t!("Open Vana", "Vana'yı Aç"),
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", t!("Quit", "Çıkış"), true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&status_item, &separator, &show, &quit])?;

    let icon = TrayIconBuilder::with_id("vana")
        .icon(Image::from_bytes(FRAMES[0])?)
        .icon_as_template(cfg!(target_os = "macos"))
        .tooltip("Vana")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => crate::show_main_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;

    app.manage(TrayState {
        icon,
        status_item,
        show_item: show,
        quit_item: quit,
        frame: Mutex::new(0),
        status: Mutex::new((0, 0)),
    });
    Ok(())
}

pub fn update(app: &AppHandle, active: usize, percent: f64) {
    let Some(state) = app.try_state::<TrayState>() else {
        return;
    };

    let target = if active == 0 { 0 } else { frame_for(percent) };
    {
        let mut frame = state.frame.lock().unwrap();
        if *frame != target {
            *frame = target;
            if let Ok(image) = Image::from_bytes(FRAMES[target]) {
                let _ = state.icon.set_icon(Some(image));
                let _ = state.icon.set_icon_as_template(cfg!(target_os = "macos"));
            }
        }
    }

    let whole = percent.round() as u32;
    let mut status = state.status.lock().unwrap();
    if *status != (active, whole) {
        *status = (active, whole);
        set_status(&state, active, whole);
        #[cfg(target_os = "macos")]
        let _ = state
            .icon
            .set_title((active > 0).then(|| t!("{whole}%", "%{whole}")));
    }
}

fn status_text(active: usize, whole: u32) -> String {
    if active == 0 {
        t!("No active downloads", "Aktif indirme yok")
    } else {
        t!(
            "{active} downloading · {whole}%",
            "{active} indirme · %{whole}"
        )
    }
}

fn set_status(state: &TrayState, active: usize, whole: u32) {
    let text = status_text(active, whole);
    let _ = state.status_item.set_text(&text);
    let _ = state.icon.set_tooltip(Some(format!("Vana — {text}")));
}

/// Applies the current language to the menu.
pub fn relabel(app: &AppHandle) {
    let Some(state) = app.try_state::<TrayState>() else {
        return;
    };
    let _ = state.show_item.set_text(t!("Open Vana", "Vana'yı Aç"));
    let _ = state.quit_item.set_text(t!("Quit", "Çıkış"));
    let (active, whole) = *state.status.lock().unwrap();
    set_status(&state, active, whole);
}

#[cfg(test)]
mod tests {
    use super::frame_for;

    #[test]
    fn frame_advances_with_progress_and_wraps() {
        assert_eq!(frame_for(0.0), 0);
        assert_eq!(frame_for(2.1), 1);
        assert_eq!(frame_for(25.0), 0);
        assert_eq!(frame_for(50.0), 0);
        assert_eq!(frame_for(52.1), 1);
        assert_eq!(frame_for(120.0), 0);
    }
}
