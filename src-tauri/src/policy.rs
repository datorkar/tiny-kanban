//! Lets IT turn off self-updates, for managed machines where IT deploys
//! new versions itself. Updates are off if any of these is set:
//!
//! - Windows registry, DWORD `DisableUpdates` = 1 under any of
//!   `HKLM\SOFTWARE\Policies\TinyKanban` (Group Policy / Intune),
//!   `HKCU\SOFTWARE\Policies\TinyKanban`,
//!   `HKLM\SOFTWARE\TinyKanban` (written by `msiexec ... DISABLEUPDATES=1`),
//!   `HKCU\SOFTWARE\TinyKanban` (written by `setup.exe /DISABLEUPDATES`).
//! - A file named `disable-updates` in `%ProgramData%\TinyKanban\` (Windows),
//!   `/Library/Application Support/TinyKanban/` (macOS) or `/etc/tiny-kanban/`
//!   (Linux); its contents don't matter.
//! - The environment variable `TINY_KANBAN_DISABLE_UPDATES` set to anything but 0.

use std::path::PathBuf;

pub fn updates_disabled() -> bool {
    if std::env::var_os("TINY_KANBAN_DISABLE_UPDATES").is_some_and(|v| v != "0") {
        return true;
    }
    if marker_file().is_some_and(|p| p.exists()) {
        return true;
    }
    registry()
}

fn marker_file() -> Option<PathBuf> {
    #[cfg(windows)]
    return std::env::var_os("ProgramData").map(|d| PathBuf::from(d).join("TinyKanban").join("disable-updates"));
    #[cfg(target_os = "macos")]
    return Some(PathBuf::from("/Library/Application Support/TinyKanban/disable-updates"));
    #[cfg(not(any(windows, target_os = "macos")))]
    return Some(PathBuf::from("/etc/tiny-kanban/disable-updates"));
}

#[cfg(windows)]
fn registry() -> bool {
    use windows::core::{w, HSTRING};
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{
        RegGetValueW, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_REG_DWORD, RRF_SUBKEY_WOW6432KEY,
        RRF_SUBKEY_WOW6464KEY,
    };
    let keys = [
        (HKEY_LOCAL_MACHINE, r"SOFTWARE\Policies\TinyKanban"),
        (HKEY_CURRENT_USER, r"SOFTWARE\Policies\TinyKanban"),
        (HKEY_LOCAL_MACHINE, r"SOFTWARE\TinyKanban"),
        (HKEY_CURRENT_USER, r"SOFTWARE\TinyKanban"),
    ];
    for (root, key) in keys {
        // Check both registry views, whichever one an installer or policy wrote to.
        for view in [RRF_SUBKEY_WOW6464KEY, RRF_SUBKEY_WOW6432KEY] {
            let (mut value, mut size) = (0u32, 4u32);
            let r = unsafe {
                RegGetValueW(
                    root,
                    &HSTRING::from(key),
                    w!("DisableUpdates"),
                    RRF_RT_REG_DWORD | view,
                    None,
                    Some(&mut value as *mut u32 as *mut core::ffi::c_void),
                    Some(&mut size),
                )
            };
            if r == ERROR_SUCCESS && value != 0 {
                return true;
            }
        }
    }
    false
}

#[cfg(not(windows))]
fn registry() -> bool {
    false
}
