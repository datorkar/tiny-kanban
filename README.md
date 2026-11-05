# Tiny Kanban

A tiny, fast, cross-platform desktop kanban board. Three columns, one-sentence cards, drag and drop. That's it.

Built with [Tauri 2](https://tauri.app): a ~3.5 MB Rust binary using the OS's own webview
(WebKit on macOS, WebView2 on Windows, WebKitGTK on Linux). The UI is a single
dependency-free HTML file (~8 KB), with no framework and no bundler.

## Usage

- **Add**: type in "+ Add a card" at the bottom of a column and press Enter. Focus stays put, so you can add several in a row.
- **Move**: drag a card within a column or to another column.
- **Edit**: double-click a card. Enter saves, Esc cancels, and clearing the text deletes the card.
- **Delete**: hover over a card and click ×.

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

Builds are unsigned. On macOS, right-click the app and choose Open the first time. On Windows, choose
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
