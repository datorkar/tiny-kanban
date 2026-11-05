#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{fs, path::PathBuf, sync::Mutex};
use tauri::{AppHandle, Manager, State};
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

// ---- App -----------------------------------------------------------------

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(Pending(Mutex::new(None)))
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
        .invoke_handler(tauri::generate_handler![load, save, check_update, install_update])
        .run(tauri::generate_context!())
        .expect("failed to run tiny-kanban");
}
