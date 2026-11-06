# Linux

Linux builds ship as **`.deb`** (Debian, Ubuntu, Mint, Pop!_OS, …) and **`.rpm`** (Fedora, RHEL, …), about
2.5 MB each.

- **Auto-update:** works for the `.deb`; you'll be asked for your password to install the update. The `.rpm`, or a
  binary copied out of a package by hand, doesn't update itself. Download new versions from
  [Releases](https://github.com/datorkar/tiny-kanban/releases).
- **Wayland:** apps can't register system-wide shortcuts or see the mouse outside their own window. Under Wayland the
  shortcuts don't work, and the lock blocks input but doesn't pass clicks through. Both work under X11.
- **See-through window:** needs a compositing desktop. GNOME, KDE and the other common desktops all have one.
- **Spell check:** follows your `LANG` setting (e.g. `en_US`) and needs a Hunspell dictionary, e.g. `hunspell-en-us`.
  Most desktop distros ship one.

## Why there's no AppImage

Tiny Kanban doesn't ship its own browser engine. It uses the system's WebKitGTK, which is why it's so small. An
AppImage has to work without relying on system libraries, so it bundles its own copy of WebKitGTK, GTK and their
dependencies. That makes it roughly **80 MB** instead of 2.5 MB, starts slower, and misses your distro's WebKit
security updates.

If you need one anyway, you can build it yourself:

```bash
npx tauri build --bundles appimage
```

## Running on other distros

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

**2a. Use the prebuilt binary from the `.deb`.** It's a self-contained executable.

```bash
mkdir tk && cd tk
ar x ../tiny-kanban_*_amd64.deb data.tar.gz   # or: bsdtar -xf ../tiny-kanban_*.deb
tar xf data.tar.gz
sudo cp usr/bin/tiny-kanban /usr/local/bin/
sudo cp "usr/share/applications/Tiny Kanban.desktop" /usr/share/applications/   # optional: app menu entry
sudo cp -r usr/share/icons/hicolor /usr/share/icons/                             # optional: icon
```

On openSUSE you can also install the `.rpm` with `sudo rpm -i --nodeps tiny-kanban-*.rpm`. Its listed dependency
names follow Fedora's package naming, but the actual libraries are the same.

**2b. Or build from source.** Install Rust, Node, and the
[Tauri Linux prerequisites](https://tauri.app/start/prerequisites/#linux) for your distro, then:

```bash
npm install
npx tauri build --no-bundle      # binary: src-tauri/target/release/tiny-kanban
```
