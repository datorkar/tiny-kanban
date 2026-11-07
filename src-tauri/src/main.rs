#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod efficiency;
mod timers;

use std::{
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, State, WebviewWindow};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tauri_plugin_window_state::StateFlags;
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

/// Lock state shared with the watcher thread.
#[derive(Default)]
struct Passthrough {
    /// Generation counter (each call starts a fresh watcher and retires the
    /// old one) and the area that must stay clickable.
    lock: Arc<Mutex<(u64, Option<[f64; 4]>)>>,
    /// Window moved, resized or changed screens: re-read position and scale.
    stale: Arc<AtomicBool>,
    /// Window minimised: stop checking until it's back.
    hidden: Arc<AtomicBool>,
    /// The current watcher, to wake it early (unlock, restore, move).
    thread: Mutex<Option<std::thread::Thread>>,
}

impl Passthrough {
    fn wake(&self) {
        if let Some(t) = &*self.thread.lock().unwrap() {
            t.unpark();
        }
    }
}

/// Keeps the watcher's cached geometry current; called from window events.
fn passthrough_event(app: &AppHandle, event: &tauri::WindowEvent) {
    use tauri::WindowEvent::*;
    if !matches!(event, Moved(_) | Resized(_) | ScaleFactorChanged { .. }) {
        return;
    }
    let pt = app.state::<Passthrough>();
    pt.stale.store(true, Ordering::Relaxed);
    if let (Resized(_), Some(w)) = (event, app.get_webview_window("main")) {
        pt.hidden.store(w.is_minimized().unwrap_or(false), Ordering::Relaxed);
    }
    pt.wake();
}

/// Click-through while locked. `keep` is the lock button's rect in CSS pixels
/// (x, y, w, h); clicks everywhere else go to the windows behind. The OS can
/// only make the whole window click-through, so a watcher thread follows the
/// cursor and makes the window clickable again while it's over the button.
/// To stay cheap it asks only for the cursor position (window position, size
/// and scale are cached and refreshed on window events), checks 4x a second
/// while the cursor is away from the window and ~30x a second over it, and
/// sleeps while the window is minimised.
/// Returns false where the cursor can't be tracked outside the window
/// (Wayland), and the caller then keeps a plain, non-click-through lock.
#[tauri::command]
fn passthrough(window: WebviewWindow, state: State<'_, Passthrough>, keep: Option<[f64; 4]>) -> bool {
    let gen = {
        let mut s = state.lock.lock().unwrap();
        s.0 += 1;
        s.1 = keep;
        s.0
    };
    state.wake(); // let a previous watcher notice it's retired
    let wayland = cfg!(target_os = "linux") && std::env::var_os("WAYLAND_DISPLAY").is_some();
    if keep.is_none() || wayland || window.cursor_position().is_err() {
        state.lock.lock().unwrap().1 = None;
        let _ = window.set_ignore_cursor_events(false);
        return keep.is_none();
    }
    let (shared, stale, hidden) = (state.lock.clone(), state.stale.clone(), state.hidden.clone());
    stale.store(true, Ordering::Relaxed);
    let handle = std::thread::spawn(move || {
        let mut ignoring = None;
        let (mut origin, mut size, mut sf) = (PhysicalPosition::new(0, 0), PhysicalSize::new(0, 0), 1.0);
        loop {
            let rect = match *shared.lock().unwrap() {
                (g, Some(r)) if g == gen => r,
                _ => break,
            };
            if hidden.load(Ordering::Relaxed) {
                std::thread::park_timeout(Duration::from_secs(5)); // woken on restore
                continue;
            }
            if stale.swap(false, Ordering::Relaxed) {
                if let (Ok(p), Ok(s), Ok(f)) = (window.inner_position(), window.inner_size(), window.scale_factor()) {
                    (origin, size, sf) = (p, s, f);
                }
            }
            let mut near = false;
            if let Ok(c) = window.cursor_position() {
                let (px, py) = (c.x - origin.x as f64, c.y - origin.y as f64);
                near = px >= 0.0 && py >= 0.0 && px < size.width as f64 && py < size.height as f64;
                let (x, y) = (px / sf, py / sf);
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
            // A 26px button can't be reached from outside the window in under
            // 250ms, so a slow check while away is just as reliable.
            std::thread::park_timeout(Duration::from_millis(if near { 30 } else { 250 }));
        }
    });
    *state.thread.lock().unwrap() = Some(handle.thread().clone());
    true
}

// ---- Launch on login --------------------------------------------------------

/// Reads (and with `on`, first changes) whether the app starts at login.
#[tauri::command]
fn autostart(app: AppHandle, on: Option<bool>) -> Result<bool, String> {
    let m = app.autolaunch();
    if let Some(on) = on {
        if on { m.enable() } else { m.disable() }.map_err(|e| e.to_string())?;
    }
    m.is_enabled().map_err(|e| e.to_string())
}

// ---- Global shortcuts ---------------------------------------------------------

/// (Re)registers the system-wide shortcuts, e.g. "CmdOrCtrl+Alt+KeyL"; an empty
/// string turns that one off. Errors name the shortcuts that couldn't be taken
/// (usually because another app already uses them).
#[tauri::command]
fn set_shortcuts(app: AppHandle, lock: String, mv: String) -> Result<(), String> {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    let mut errs = Vec::new();
    for (keys, what) in [(lock, "lock"), (mv, "move")] {
        if keys.is_empty() {
            continue;
        }
        let r = gs.on_shortcut(keys.as_str(), move |app, _, ev| {
            if ev.state != ShortcutState::Pressed {
                return;
            }
            let Some(w) = app.get_webview_window("main") else { return };
            if what == "move" {
                move_to_cursor(&w);
            } else {
                let _ = w.emit("shortcut", what); // the page owns the lock state
            }
        });
        if let Err(e) = r {
            errs.push(format!("{what}: {e}"));
        }
    }
    if errs.is_empty() { Ok(()) } else { Err(errs.join("; ")) }
}

/// Puts the window's top strip under the mouse, kept on the mouse's screen.
fn move_to_cursor(w: &WebviewWindow) {
    // inner == outer for this borderless window, and outer_size can read 0 on Linux
    let (Ok(c), Ok(size)) = (w.cursor_position(), w.inner_size()) else { return };
    let sf = w.scale_factor().unwrap_or(1.0);
    let (ww, wh) = (size.width as f64, size.height as f64);
    let (mut x, mut y) = (c.x - ww / 2.0, c.y - 11.0 * sf);
    let monitor = w.monitor_from_point(c.x, c.y).ok().flatten().or_else(|| w.current_monitor().ok().flatten());
    if let Some(m) = monitor {
        let (p, s) = (m.position(), m.size());
        x = x.min(p.x as f64 + s.width as f64 - ww).max(p.x as f64);
        y = y.min(p.y as f64 + s.height as f64 - wh).max(p.y as f64);
    }
    let _ = w.unminimize();
    let _ = w.set_position(PhysicalPosition::new(x.round() as i32, y.round() as i32));
}

// ---- App -----------------------------------------------------------------

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        // Remembers window position and size, saved on close and on exit.
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(StateFlags::SIZE | StateFlags::POSITION | StateFlags::MAXIMIZED)
                .build(),
        )
        .manage(Pending(Mutex::new(None)))
        .manage(Passthrough::default())
        .manage(timers::Timers::default())
        .setup(|_app| {
            if let Some(win) = _app.get_webview_window("main") {
                efficiency::trim_after_start(win);
            }
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
                        // One small page: keep WebKit's caches minimal.
                        ctx.set_cache_model(webkit2gtk::CacheModel::DocumentViewer);
                    }
                });
            }
            Ok(())
        })
        // Save power while the window isn't focused; full speed when it is.
        .on_window_event(|window, event| {
            passthrough_event(window.app_handle(), event);
            if let tauri::WindowEvent::Focused(focused) = event {
                if let Some(w) = window.app_handle().get_webview_window("main") {
                    efficiency::set(&w, !*focused);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            load,
            save,
            check_update,
            install_update,
            set_on_top,
            passthrough,
            autostart,
            set_shortcuts,
            timers::set_timers
        ])
        .run(tauri::generate_context!())
        .expect("failed to run tiny-kanban");
}
