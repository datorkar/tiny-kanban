"""Build the updater manifest (latest.json) from the signed bundles in DIST.

Keys are "<os>-<arch>-<installer>", matching update_target() in
src-tauri/src/main.rs, so every install updates with its own installer type.
"""
import datetime, json, pathlib, sys

dist, repo, tag = pathlib.Path(sys.argv[1]), sys.argv[2], sys.argv[3]
kinds = {  # file suffix -> latest.json keys
    "_amd64.deb": ["linux-x86_64-deb"],
    "-setup.exe": ["windows-x86_64-nsis"],
    ".msi": ["windows-x86_64-msi"],
    ".app.tar.gz": ["darwin-aarch64-app", "darwin-x86_64-app"],  # universal build
}
platforms = {}
for sig in dist.rglob("*.sig"):
    asset = sig.name[: -len(".sig")]
    for suffix, keys in kinds.items():
        if asset.endswith(suffix):
            entry = {
                "signature": sig.read_text().strip(),
                "url": f"https://github.com/{repo}/releases/download/{tag}/{asset}",
            }
            platforms.update({k: entry for k in keys})

missing = {k for ks in kinds.values() for k in ks} - platforms.keys()
if missing:
    sys.exit(f"No signed bundle for {sorted(missing)}. Is the TAURI_SIGNING_PRIVATE_KEY secret set?")

json.dump(
    {
        "version": tag.lstrip("v"),
        "notes": f"https://github.com/{repo}/releases/tag/{tag}",
        "pub_date": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "platforms": platforms,
    },
    sys.stdout,
    indent=2,
)
