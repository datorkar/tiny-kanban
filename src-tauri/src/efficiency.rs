//! Use as little power as possible while the window isn't focused, and run at
//! full speed again the moment it is (so typing and dragging stay snappy).
//!
//! - Windows: "efficiency mode" (EcoQoS + idle priority, the leaf icon in Task
//!   Manager) for the app and the WebView2 processes it started, which do
//!   nearly all the work, and WebView2 is asked to trim its memory.
//! - macOS: the app process runs as background work (efficiency cores,
//!   throttled I/O). WebKit's own web-content process is managed by macOS.
//! - Linux: nothing to switch; WebKit's caches are kept small at start-up
//!   instead (see main.rs).

use std::sync::atomic::{AtomicU8, Ordering};
use tauri::WebviewWindow;

/// Last applied state (0 = unset, 1 = normal, 2 = low), to skip repeats.
static STATE: AtomicU8 = AtomicU8::new(0);

pub fn set(window: &WebviewWindow, low: bool) {
    if STATE.swap(if low { 2 } else { 1 }, Ordering::Relaxed) == if low { 2 } else { 1 } {
        return;
    }
    apply(window, low);
}

#[cfg(windows)]
fn apply(window: &WebviewWindow, low: bool) {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
    };
    use windows::Win32::System::Threading::{
        GetCurrentProcessId, OpenProcess, ProcessPowerThrottling, SetPriorityClass, SetProcessInformation,
        IDLE_PRIORITY_CLASS, NORMAL_PRIORITY_CLASS, PROCESS_POWER_THROTTLING_CURRENT_VERSION,
        PROCESS_POWER_THROTTLING_EXECUTION_SPEED, PROCESS_POWER_THROTTLING_STATE, PROCESS_SET_INFORMATION,
    };

    // This process and its descendants (WebView2 browser, renderer, GPU, ...).
    let pids = unsafe {
        let mut pids = vec![GetCurrentProcessId()];
        if let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) {
            let mut all = Vec::new();
            let mut e = PROCESSENTRY32W { dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
            if Process32FirstW(snap, &mut e).is_ok() {
                loop {
                    all.push((e.th32ProcessID, e.th32ParentProcessID));
                    if Process32NextW(snap, &mut e).is_err() {
                        break;
                    }
                }
            }
            let _ = CloseHandle(snap);
            let mut i = 0;
            while i < pids.len() {
                let parent = pids[i];
                for &(pid, ppid) in &all {
                    if ppid == parent && pid != 0 && !pids.contains(&pid) {
                        pids.push(pid);
                    }
                }
                i += 1;
            }
        }
        pids
    };

    // Low: EcoQoS on. Normal: clear our override and let Windows decide again.
    let mask = if low { PROCESS_POWER_THROTTLING_EXECUTION_SPEED } else { 0 };
    let state = PROCESS_POWER_THROTTLING_STATE {
        Version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
        ControlMask: mask,
        StateMask: mask,
    };
    for pid in pids {
        unsafe {
            let Ok(h) = OpenProcess(PROCESS_SET_INFORMATION, false, pid) else { continue };
            let _ = SetProcessInformation(
                h,
                ProcessPowerThrottling,
                &state as *const _ as *const core::ffi::c_void,
                std::mem::size_of_val(&state) as u32,
            );
            let _ = SetPriorityClass(h, if low { IDLE_PRIORITY_CLASS } else { NORMAL_PRIORITY_CLASS });
            let _ = CloseHandle(h);
        }
    }

    memory_target(window, low);
}

/// WebView2's memory target: low makes it trim caches and give memory back.
#[cfg(windows)]
fn memory_target(window: &WebviewWindow, low: bool) {
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        ICoreWebView2_19, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL,
    };
    use windows::core::Interface;
    let _ = window.with_webview(move |wv| unsafe {
        if let Ok(core) = wv.controller().CoreWebView2() {
            if let Ok(wv19) = core.cast::<ICoreWebView2_19>() {
                let _ = wv19.SetMemoryUsageTargetLevel(if low {
                    COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW
                } else {
                    COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL
                });
            }
        }
    });
}

/// Chromium allocates a lot while starting up (~80 MB on Windows) and keeps it
/// until asked to trim, which otherwise only happens on the first unfocus. So a
/// few seconds after start, trim once, then go back to the normal target
/// (unless the window has gone to the background meanwhile).
pub fn trim_after_start(window: WebviewWindow) {
    #[cfg(windows)]
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(4));
        if STATE.load(Ordering::Relaxed) == 2 {
            return; // already in the background, so already trimmed
        }
        memory_target(&window, true);
        std::thread::sleep(std::time::Duration::from_secs(2));
        if STATE.load(Ordering::Relaxed) != 2 {
            memory_target(&window, false);
        }
    });
    #[cfg(not(windows))]
    let _ = window;
}

#[cfg(target_os = "macos")]
fn apply(_: &WebviewWindow, low: bool) {
    unsafe {
        libc::setpriority(libc::PRIO_DARWIN_PROCESS, 0, if low { libc::PRIO_DARWIN_BG } else { 0 });
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
fn apply(_: &WebviewWindow, _: bool) {}
