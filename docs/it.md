# For IT

Facts and settings for deploying Tiny Kanban on managed machines.

## What it does on the machine

| | |
|---|---|
| **Network** | Only GitHub: an update check at start and every 6 hours (`github.com/datorkar/tiny-kanban/releases/latest/download/latest.json`), and the update download. No telemetry, account or cloud. With updates turned off (below), it makes no network requests. Web links in cards open in the user's browser. |
| **Data** | The board is plain JSON in the user's profile: `%APPDATA%\app.tinykanban\board.json` (Windows, roams with the profile), `~/Library/Application Support/app.tinykanban/` (macOS), `~/.local/share/app.tinykanban/` (Linux). Other boards are in a `boards` folder beside it. Each board file gets a daily `.bak` copy next to it. WebView2's cache is in `%LOCALAPPDATA%\app.tinykanban\EBWebView`. |
| **Privileges** | Runs as the user; nothing needs admin. The `.msi` installs per machine, the `.exe` per user. |
| **Startup** | Only if the user ticks *Launch at login*: an entry in `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` (Windows), a LaunchAgent (macOS) or `~/.config/autostart` (Linux). |
| **Keyboard** | Two system-wide shortcuts (default Ctrl+Alt+L / Ctrl+Alt+M) via the standard hotkey API (`RegisterHotKey`), not keyboard hooks. Users can change or turn them off in settings. |
| **Mouse** | While the board is locked as a click-through overlay, it reads the cursor position (4–30 times a second) to keep its lock button clickable. |
| **Other processes** | When unfocused, it puts its own WebView2 processes (`msedgewebview2.exe`) into Windows efficiency mode. It doesn't touch any other process. |
| **Runtime** | The system WebView2 (Windows 10/11), WebKit (macOS) or WebKitGTK 4.1 (Linux). No bundled browser. |

## Silent install

```powershell
msiexec /i tiny-kanban_<version>_x64_en-US.msi /qn
tiny-kanban_<version>_x64-setup.exe /S
```

## Turning off self-updates

By default the app updates itself from GitHub Releases (signed updates only). On managed machines you can turn that off and deploy new versions yourself. Any one of these works:

| Method | How |
|---|---|
| **Group Policy / Intune** (recommended) | DWORD `DisableUpdates` = `1` in `HKLM\SOFTWARE\Policies\TinyKanban` (or `HKCU\...` for a user) |
| **MSI install** | `msiexec /i tiny-kanban_<version>_x64_en-US.msi DISABLEUPDATES=1 /qn` (writes `HKLM\SOFTWARE\TinyKanban\DisableUpdates`; removed on uninstall, so pass it on every upgrade) |
| **EXE install** | `tiny-kanban_<version>_x64-setup.exe /S /DISABLEUPDATES` (writes `Software\TinyKanban\DisableUpdates` under HKCU, or HKLM for a per-machine install; pass it on every upgrade) |
| **File** (any OS) | Create an empty file named `disable-updates` in `%ProgramData%\TinyKanban\`, `/Library/Application Support/TinyKanban/` (macOS) or `/etc/tiny-kanban/` (Linux) |
| **Environment** | `TINY_KANBAN_DISABLE_UPDATES=1` |

The app then never contacts GitHub, and its settings show *Updates are managed by your organisation*.

## Verifying a download

Each release has a signed build provenance attestation, which proves the files were built by this repository's GitHub Actions workflow from a specific commit:

```bash
gh attestation verify tiny-kanban_<version>_x64-setup.exe -R datorkar/tiny-kanban
```

Each release also includes an SPDX software bill of materials (`tiny-kanban-v<version>.spdx.json`) listing every Rust and npm dependency. Dependencies are audited weekly against the RustSec advisory database.

The installers aren't Authenticode/Apple code-signed yet. Self-updates are verified with the project's own update-signing key.

## Licence

GPL-3.0 for the app itself. Dependencies are MIT/Apache-2.0/BSD, plus MPL-2.0 for a few unmodified libraries that come with Tauri.
