#!/usr/bin/env python3
"""Generate the Tauri updater manifest (latest.json) for a GitHub release.

Scans a directory of release artifacts for updater bundles and their minisign
signatures, then writes `latest.json` into that directory so it is uploaded
(and checksummed) alongside the other assets.

Updater bundles per platform (Tauri v2 `createUpdaterArtifacts`):
  linux-x86_64    *.AppImage      + *.AppImage.sig
  darwin-aarch64  *.app.tar.gz    + *.app.tar.gz.sig   (built on arm64 runners)
  windows-x86_64  *-setup.exe     + *-setup.exe.sig    (NSIS; falls back to .msi)

The manifest also includes a `linux_packages` block carrying the non-updater
.deb and .rpm asset URLs + SHA-256 for the in-app download-and-handoff path.

Usage: generate-latest-json.py <artifacts-dir> <tag>
"""

import datetime
import hashlib
import json
import pathlib
import sys

REPO_DOWNLOAD_BASE = "https://github.com/visorcraft/Kanoprii/releases/download"


def github_asset_name(filename: str) -> str:
    """GitHub normalizes release asset names; spaces become dots."""
    return filename.replace(" ", ".")


def find_bundle(root: pathlib.Path, suffix: str) -> pathlib.Path | None:
    matches = sorted(
        p for p in root.rglob(f"*{suffix}") if not p.name.endswith(".sig")
    )
    return matches[0] if matches else None


def platform_entry(root: pathlib.Path, tag: str, suffixes: list[str]) -> dict | None:
    for suffix in suffixes:
        bundle = find_bundle(root, suffix)
        if bundle is None:
            continue
        sig = bundle.with_name(bundle.name + ".sig")
        if not sig.is_file():
            raise SystemExit(
                f"error: found {bundle.name} but no {sig.name}; "
                "was TAURI_SIGNING_PRIVATE_KEY set during the build?"
            )
        return {
            "signature": sig.read_text().strip(),
            "url": f"{REPO_DOWNLOAD_BASE}/{tag}/{github_asset_name(bundle.name)}",
        }
    return None


def sha256_hex(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as f:
        while True:
            chunk = f.read(65536)
            if not chunk:
                break
            digest.update(chunk)
    return digest.hexdigest()


def linux_packages(root: pathlib.Path, tag: str) -> dict:
    """deb/rpm asset URL + checksum for the in-app download-and-handoff path."""
    out: dict = {}
    for key, suffix in (("deb", ".deb"), ("rpm", ".rpm")):
        bundle = find_bundle(root, suffix)
        if bundle is None:
            continue
        out[key] = {
            "url": f"{REPO_DOWNLOAD_BASE}/{tag}/{github_asset_name(bundle.name)}",
            "sha256": sha256_hex(bundle),
        }
    return out


def main() -> None:
    if len(sys.argv) != 3:
        raise SystemExit(f"usage: {sys.argv[0]} <artifacts-dir> <tag>")
    root = pathlib.Path(sys.argv[1])
    tag = sys.argv[2]
    if not root.is_dir():
        raise SystemExit(f"error: {root} is not a directory")

    platforms = {}
    for key, suffixes in {
        "linux-x86_64": [".AppImage"],
        "darwin-aarch64": [".app.tar.gz"],
        "windows-x86_64": ["-setup.exe", ".msi"],
    }.items():
        entry = platform_entry(root, tag, suffixes)
        if entry is None:
            print(f"warning: no updater bundle found for {key}", file=sys.stderr)
            continue
        platforms[key] = entry

    if not platforms:
        raise SystemExit("error: no updater bundles found in artifacts directory")

    manifest = {
        "version": tag.lstrip("v"),
        "notes": f"Kanoprii {tag}",
        "pub_date": datetime.datetime.now(datetime.timezone.utc)
        .replace(microsecond=0)
        .isoformat()
        .replace("+00:00", "Z"),
        "platforms": platforms,
    }
    packages = linux_packages(root, tag)
    if packages:
        manifest["linux_packages"] = packages
    out = root / "latest.json"
    out.write_text(json.dumps(manifest, indent=2) + "\n")
    print(f"Wrote {out} ({', '.join(sorted(platforms))})")


if __name__ == "__main__":
    main()
