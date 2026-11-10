//! Card timers and deadlines. One thread sleeps until the next timer is due (no CPU use in
//! between, and unaffected by the webview slowing timers in hidden windows),
//! then shows an OS notification. If that can't be delivered, it restores the
//! window and tells the page, which plays a ping and shows itself at full
//! opacity.

use std::{
    sync::{Arc, Mutex},
    thread::Thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter, Manager, State};

/// (card id, due time in ms since the epoch, notification title, card text)
type Timer = (String, u64, String, String);

#[derive(Default)]
pub struct Timers {
    list: Arc<Mutex<Vec<Timer>>>,
    /// Already fired (id, due), so a list sent again before the page caught up
    /// doesn't notify twice.
    fired: Arc<Mutex<Vec<(String, u64)>>>,
    thread: Mutex<Option<Thread>>,
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

/// Replaces the set of running timers (the page sends all of them).
#[tauri::command]
pub fn set_timers(app: AppHandle, state: State<'_, Timers>, list: Vec<Timer>) {
    {
        let fired = state.fired.lock().unwrap();
        *state.list.lock().unwrap() = list.into_iter().filter(|(id, due, ..)| !fired.contains(&(id.clone(), *due))).collect();
    }
    let mut thread = state.thread.lock().unwrap();
    if let Some(t) = &*thread {
        t.unpark();
        return;
    }
    let (list, fired) = (state.list.clone(), state.fired.clone());
    let handle = std::thread::spawn(move || loop {
        let now = now_ms();
        let due: Vec<Timer> = {
            let mut l = list.lock().unwrap();
            let (due, keep) = l.drain(..).partition(|t| t.1 <= now);
            *l = keep;
            due
        };
        for (id, at, title, text) in due {
            fired.lock().unwrap().push((id.clone(), at));
            let notified = notify(&app, &title, &text);
            if !notified {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.unminimize();
                    let _ = w.show();
                }
            }
            let _ = app.emit("timer", (id, notified));
        }
        let next = list.lock().unwrap().iter().map(|t| t.1).min();
        match next {
            // Re-check at least once a minute, in case the system clock changes.
            Some(at) => std::thread::park_timeout(Duration::from_millis(at.saturating_sub(now_ms()).clamp(1, 60_000))),
            None => std::thread::park(),
        }
    });
    *thread = Some(handle.thread().clone());
}

/// Shows the OS notification; false if it couldn't be delivered.
fn notify(app: &AppHandle, title: &str, text: &str) -> bool {
    let mut n = notify_rust::Notification::new();
    n.summary(title).body(text).auto_icon();
    #[cfg(windows)]
    {
        // Attribute the toast to the installed app (the installer registers this
        // id); a bare binary from target/ has no registration, so use the default.
        if let Ok(exe) = std::env::current_exe() {
            let dir = exe.parent().map(|d| d.display().to_string()).unwrap_or_default();
            if !(dir.ends_with("target\\release") || dir.ends_with("target\\debug")) {
                n.app_id(&app.config().identifier);
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        let _ = notify_rust::set_application(&app.config().identifier);
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    let _ = app;
    n.show().is_ok()
}
