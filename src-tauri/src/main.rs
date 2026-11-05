#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
use tauri::{AppHandle, Manager, State, WebviewWindow};
use tauri_plugin_updater::{Update, UpdaterExt};

fn board_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("board.json"))
}

#[tauri::command]
fn load(app: AppHandle) -> String {
    board_path(&app)
        .ok()
        .and_then(|p| fs::read_to_string(p).ok())
        .unwrap_or_default()
}

/// Write atomically (tmp file + rename) so a crash never corrupts the board.
#[tauri::command]
fn save(app: AppHandle, data: String) -> Result<(), String> {
    let path = board_path(&app)?;
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, data).map_err(|e| e.to_string())?;
    fs::rename(tmp, path).map_err(|e| e.to_string())
}

// ---- Auto-update ---------------------------------------------------------

struct Pending(Mutex<Option<Update>>);

/// Key into latest.json for the way this copy was installed, e.g.
/// "windows-x86_64-msi", so updates use the same installer type.
/// None means this copy can't update itself: .rpm installs (the updater
/// only knows dpkg), or a binary that was copied out of a package by hand.
fn update_target() -> Option<String> {
    use tauri::utils::{config::BundleType, platform::bundle_type};
    let kind = match bundle_type()? {
        BundleType::Nsis => "nsis",
        BundleType::Msi => "msi",
        BundleType::App => "app",
        BundleType::Deb if fs::metadata("/var/lib/dpkg/info/tiny-kanban.list").is_ok() => "deb",
        _ => return None,
    };
    let os = match std::env::consts::OS {
        "macos" => "darwin",
        os => os,
    };
    Some(format!("{os}-{}-{kind}", std::env::consts::ARCH))
}

/// Returns the new version if one is available (and remembers it for install).
#[tauri::command]
async fn check_update(app: AppHandle, pending: State<'_, Pending>) -> Result<Option<String>, String> {
    let Some(target) = update_target() else { return Ok(None) };
    let update = app
        .updater_builder()
        .target(target)
        .build()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?;
    let version = update.as_ref().map(|u| u.version.clone());
    *pending.0.lock().unwrap() = update;
    Ok(version)
}

#[tauri::command]
async fn install_update(app: AppHandle, pending: State<'_, Pending>) -> Result<(), String> {
    let update = pending.0.lock().unwrap().take().ok_or("No update available")?;
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| e.to_string())?;
    app.restart()
}

// ---- Window modes ----------------------------------------------------------

#[tauri::command]
fn set_on_top(window: WebviewWindow, on: bool) -> Result<(), String> {
    window.set_always_on_top(on).map_err(|e| e.to_string())
}

/// Lock state: a generation counter (each call starts a fresh watcher and
/// retires the old one) and the area that must stay clickable.
#[derive(Default)]
struct Passthrough(Arc<Mutex<(u64, Option<[f64; 4]>)>>);

/// Click-through while locked. `keep` is the lock button's rect in CSS pixels
/// (x, y, w, h); clicks everywhere else go to the windows behind. The OS can
/// only make the whole window click-through, so a watcher thread follows the
/// cursor and makes the window clickable again while it's over the button.
/// Returns false where the cursor can't be tracked outside the window
/// (Wayland), and the caller then keeps a plain, non-click-through lock.
#[tauri::command]
fn passthrough(window: WebviewWindow, state: State<'_, Passthrough>, keep: Option<[f64; 4]>) -> bool {
    let gen = {
        let mut s = state.0.lock().unwrap();
        s.0 += 1;
        s.1 = keep;
        s.0
    };
    let wayland = cfg!(target_os = "linux") && std::env::var_os("WAYLAND_DISPLAY").is_some();
    if keep.is_none() || wayland || window.cursor_position().is_err() {
        state.0.lock().unwrap().1 = None;
        let _ = window.set_ignore_cursor_events(false);
        return keep.is_none();
    }
    let shared = state.0.clone();
    std::thread::spawn(move || {
        let mut ignoring = None;
        loop {
            let rect = match *shared.lock().unwrap() {
                (g, Some(r)) if g == gen => r,
                _ => break,
            };
            if let (Ok(c), Ok(p), Ok(sf)) = (window.cursor_position(), window.inner_position(), window.scale_factor()) {
                let (x, y) = ((c.x - p.x as f64) / sf, (c.y - p.y as f64) / sf);
                let over = x >= rect[0] && x < rect[0] + rect[2] && y >= rect[1] && y < rect[1] + rect[3];
                if ignoring != Some(!over) {
                    let _ = window.set_ignore_cursor_events(!over);
                    ignoring = Some(!over);
                    // Unlocked meanwhile? Never leave the window click-through.
                    if shared.lock().unwrap().0 != gen {
                        let _ = window.set_ignore_cursor_events(false);
                        break;
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(30));
        }
    });
    true
}

// ---- App -----------------------------------------------------------------

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(Pending(Mutex::new(None)))
        .manage(Passthrough::default())
        .setup(|_app| {
            // macOS and Windows webviews spell-check out of the box; WebKitGTK
            // needs it switched on and told which language to use.
            #[cfg(target_os = "linux")]
            if let Some(win) = _app.get_webview_window("main") {
                let _ = win.with_webview(|wv| {
                    use webkit2gtk::{WebContextExt, WebViewExt};
                    if let Some(ctx) = wv.inner().context() {
                        let lang = std::env::var("LANG")
                            .ok()
                            .and_then(|l| l.split('.').next().map(str::to_owned))
                            .filter(|l| l.len() >= 2 && l != "C" && l != "POSIX")
                            .unwrap_or_else(|| "en_US".into());
                        ctx.set_spell_checking_languages(&[lang.as_str()]);
                        ctx.set_spell_checking_enabled(true);
                    }
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            load,
            save,
            check_update,
            install_update,
            set_on_top,
            passthrough
        ])
        .run(tauri::generate_context!())
        .expect("failed to run tiny-kanban");
}
