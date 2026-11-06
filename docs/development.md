# Development

Requirements: Rust (stable), Node 18+, and the [Tauri prerequisites](https://tauri.app/start/prerequisites/) for
your OS.

```bash
npm install
npm run dev     # run with hot reload
npm run build   # release bundles in src-tauri/target/release/bundle
```

## Layout

- `src/index.html`: the whole UI (HTML, CSS and JS in one file, with no framework or bundler).
- `src-tauri/src/main.rs`: saving and loading, the updater, lock click-through, shortcuts, launch at login.
- `src-tauri/src/efficiency.rs`: power saving while the window isn't focused.
- `src-tauri/tauri.conf.json`: window, bundle and updater configuration.

Branches: work happens on `develop`, and `main` is what gets released.

## CI

GitHub Actions builds Linux (`.deb`, `.rpm`), Windows (`.msi`, `.exe`) and macOS (universal `.dmg`) on every push to
`main` and `develop`. The installers are attached to each run as artifacts.

## Releasing

1. Bump `version` in `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` and `package.json`.
2. Either push a `v<version>` tag, or go to **Actions → Build → Run workflow** on `main` and tick
   **Publish a GitHub Release**. That tags `v<version>` itself.

The release includes `latest.json`, which installed apps read to find updates. Each install type updates with its
own installer: `.exe` or `.msi` on Windows, the app on macOS, `.deb` on Linux. `.rpm` installs don't auto-update.

## Update signing

Updates are signed, and the app only installs updates signed with the project's key. CI needs the private key in
a repository secret named **`TAURI_SIGNING_PRIVATE_KEY`** (Settings → Secrets and variables → Actions). If the key
has a password, also add **`TAURI_SIGNING_PRIVATE_KEY_PASSWORD`**. Without the secret, CI still builds installers,
but a release run fails rather than publish a release that can't auto-update. The public key is in
`src-tauri/tauri.conf.json` under `plugins.updater.pubkey`.

The installers themselves aren't OS code-signed (no Apple or Windows certificate).
