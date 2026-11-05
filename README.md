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

GitHub Actions builds for Linux (.deb, .rpm, .AppImage), Windows (.msi, .exe), and macOS
(universal .dmg) on every push. You can download the installers from the run's artifacts. Pushing a `v*` tag
also attaches them to a draft GitHub Release.

Builds are unsigned. On macOS, right-click the app and choose Open the first time. On Windows, choose
"More info → Run anyway" in SmartScreen.
