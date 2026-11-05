# Tiny Kanban

A tiny, fast, cross-platform desktop kanban board. Three columns, one-sentence cards, drag and drop. That's it.

Built with [Tauri 2](https://tauri.app): a ~5 MB Rust binary using the OS's own webview
(WebKit on macOS, WebView2 on Windows, WebKitGTK on Linux). The UI is a single
dependency-free HTML file (~12 KB), with no framework and no bundler.

## Usage

- **Add**: click the **+** at the bottom of a column, type and press Enter. Focus stays put, so you can add several in a row.
- **Move**: drag a card within a column or to another column.
- **Edit**: double-click a card. Enter saves, Esc cancels, and clearing the text deletes the card.
- **Delete**: hover over a card and click ×.
- **Autocomplete**: while typing, words already on your board appear as faint text after the cursor. Press Tab or → to accept.
- **Spell check**: uses the operating system's own spell checker and dictionaries. On Linux it follows your `LANG`
  setting (e.g. `en_US`) and needs a Hunspell dictionary installed, e.g. `hunspell-en-us`. Most desktop distros ship one.
- **Mini mode**: click the dot in the middle of the top strip. Text shrinks to half size so more fits. Hovering a
  card pops out a full-size copy over the board without moving other cards; you can drag it, double-click it to edit,
  or use its × to delete. Cards also return to full size while being dragged or edited. The dot is smaller while mini mode is on, and the
  setting is remembered.
- **Always on top**: click the pin in the top strip to keep the window above all others. It's remembered across restarts.
- **Lock**: click the padlock in the top strip. While locked, the board ignores the mouse and keyboard, and clicks pass
  straight through to the windows behind it. Only the padlock still responds. Combined with always-on-top and low
  opacity, the board becomes a see-through overlay you can work underneath. The lock is remembered across restarts.
  Under Linux Wayland, apps can't track the mouse outside their own window, so there the lock still blocks input but
  doesn't pass clicks through.
- **Opacity**: right-click the top strip for a slider, 10–100% in 10% steps. It's remembered across restarts. On Linux,
  see-through needs a compositing desktop, which all the common desktops have.
- **Window**: there's no OS title bar. Drag the thin strip at the top to move the window, and double-click it to maximise.

## Updates

The app checks GitHub Releases for a new version on start and every 6 hours. When one is available, an
**Update to x.y.z** button appears in the top strip. Click it to download, install and restart.
Your board is saved first.

- **Windows / macOS**: updates in place, using the same installer type you installed with (.exe or .msi).
- **Linux .deb**: updates in place; you'll be asked for your password to install the package.
- **Linux .rpm**, or a binary copied out of a package by hand: no auto-update. Download new versions from
  [Releases](https://github.com/datorkar/tiny-kanban/releases).

The board saves automatically to `board.json` in the app data directory:

| OS      | Path |
|---------|------|
| Linux   | `~/.local/share/app.tinykanban/board.json` |
| macOS   | `~/Library/Application Support/app.tinykanban/board.json` |
| Windows | `%APPDATA%\app.tinykanban\board.json` |

## Develop

Requirements: Rust (stable), Node 18+, and the [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your OS.

```bash
npm install
npm run dev     # run with hot reload
npm run build   # release bundles in src-tauri/target/release/bundle
```

## CI

GitHub Actions builds for Linux (.deb, .rpm), Windows (.msi, .exe), and macOS
(universal .dmg) on every push. You can download the installers from the run's artifacts. Pushing a `v*` tag
also publishes them as a GitHub Release. You can do the same without a local tag: Actions → Build →
Run workflow → tick "Publish a GitHub Release". That tags `v<version>` from `src-tauri/tauri.conf.json`.

See [Linux packages](#linux-packages) for why there's no AppImage.

### Releasing

Updates are signed, and the app only installs updates signed with the project's key. CI needs the private key
in a repository secret named **`TAURI_SIGNING_PRIVATE_KEY`** (Settings → Secrets and variables → Actions).
If the key has a password, also add it as `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. Without the secret, CI still
builds installers, but a release run will fail rather than publish a release that can't auto-update.
The public key is in `src-tauri/tauri.conf.json` under `plugins.updater.pubkey`.

To release, bump `version` in `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` and `package.json`, then
push a `v<version>` tag or use Run workflow as above. The release includes `latest.json`, which installed
apps read to find the update.

The installers themselves are not OS code-signed. On macOS, right-click the app and choose Open the first time. On Windows, choose
"More info → Run anyway" in SmartScreen.

## Linux packages

Linux builds ship only as **`.deb`** (Debian, Ubuntu, Mint, Pop!_OS, …) and **`.rpm`** (Fedora, RHEL, …).
Each is about 1.4 MB.

### Why there's no AppImage

Tiny Kanban doesn't ship its own browser engine. It uses the system's WebKitGTK, which is why it's so small.
An AppImage has to work without relying on system libraries, so it bundles its own copy of WebKitGTK,
GTK and their dependencies. That makes it roughly **80 MB**, about 60× bigger, for a slower-starting app
that also misses your distro's WebKit security updates. That cost isn't worth it for a 3.5 MB app.

### Running on other distros

The app needs only two system libraries: **WebKitGTK 4.1** and **GTK 3**. The released binaries are built on
Ubuntu 22.04, so they also need **glibc 2.35 or newer**, which any distro from 2022 onward has.

**1. Install the runtime libraries**

| Distro | Command |
|---|---|
| Arch / Manjaro / EndeavourOS | `sudo pacman -S webkit2gtk-4.1` |
| openSUSE | `sudo zypper install libwebkit2gtk-4_1-0` |
| Void | `sudo xbps-install webkit2gtk` |
| Alpine | `sudo apk add webkit2gtk-4.1` |
| Gentoo | `sudo emerge net-libs/webkit-gtk:4.1` |

**2a. Use the prebuilt binary from the `.deb`** (it's a self-contained executable)

```bash
mkdir tk && cd tk
ar x ../tiny-kanban_*_amd64.deb data.tar.gz   # or: bsdtar -xf ../tiny-kanban_*.deb
tar xf data.tar.gz
sudo cp usr/bin/tiny-kanban /usr/local/bin/
sudo cp "usr/share/applications/Tiny Kanban.desktop" /usr/share/applications/   # optional: app menu entry
sudo cp -r usr/share/icons/hicolor /usr/share/icons/                             # optional: icon
```

On openSUSE you can also install the `.rpm` with `sudo rpm -i --nodeps tiny-kanban-*.rpm`. Its listed
dependency names follow Fedora's package naming, but the actual libraries are the same.

**2b. Or build from source.** Install Rust, Node, and the
[Tauri Linux prerequisites](https://tauri.app/start/prerequisites/#linux) for your distro, then:

```bash
npm install
npx tauri build --no-bundle      # binary: src-tauri/target/release/tiny-kanban
```

**Need an AppImage anyway?** You can still build one yourself:

```bash
npx tauri build --bundles appimage
```
