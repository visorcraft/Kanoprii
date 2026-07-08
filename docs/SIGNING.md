# Release Signing

Kanoprii release packages are **unsigned by default**. When signing credentials are
configured as GitHub Actions secrets, the release workflow signs macOS and Windows
artifacts automatically before publishing.

## Automated releases (GitHub Actions)

Run the **Release** workflow manually (**Actions → Release → Run workflow**). The pipeline:

1. Builds Linux deb/rpm/AppImage, macOS `.app`/`.dmg`, and Windows `.msi`/`.exe` packages,
   plus minisign-signed updater artifacts (`createUpdaterArtifacts`).
2. Signs macOS and Windows artifacts when the secrets below are present.
3. Generates the updater manifest `latest.json` (`scripts/generate-latest-json.py`)
   pointing at the tagged release's assets.
4. Writes `SHA256SUMS.txt` for all uploaded files.
5. Publishes a GitHub Release for the `tag` input with every artifact attached.

Workflow file: `.github/workflows/release.yml`

## Updater key (Tauri updater)

In-app updates verify a minisign signature against the public key embedded in
`src-tauri/tauri.conf.json` (`plugins.updater.pubkey`). The matching private key lives at
`~/.tauri/kanoprii.key`; its password lives at `~/.tauri/kanoprii.key.password`.
They are **not** in the repo; if the key is lost, generate a new pair and update the
pubkey - older installs will then refuse updates until reinstalled. The key and password
must be set as the `TAURI_SIGNING_*` secrets below before running the Release workflow.

### Required GitHub secrets

| Secret | Platform | Purpose |
| --- | --- | --- |
| `TAURI_SIGNING_PRIVATE_KEY` | all | Tauri updater private key (contents of `~/.tauri/kanoprii.key`) - **required**; builds fail without it because `createUpdaterArtifacts` is enabled |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | all | Password for the updater key (contents of `~/.tauri/kanoprii.key.password`) |
| `APPLE_CERTIFICATE_BASE64` | macOS | Base64-encoded `.p12` Developer ID Application certificate |
| `APPLE_CERTIFICATE_PASSWORD` | macOS | Password for the `.p12` import |
| `APPLE_SIGNING_IDENTITY` | macOS | `codesign` identity string (e.g. `Developer ID Application: …`) |
| `APPLE_ID` | macOS | Apple ID for `notarytool` (optional but recommended) |
| `APPLE_APP_PASSWORD` | macOS | App-specific password for notarization (optional) |
| `APPLE_TEAM_ID` | macOS | 10-character Team ID for notarization (optional) |
| `WINDOWS_CERTIFICATE_BASE64` | Windows | Base64-encoded `.pfx` code-signing certificate |
| `WINDOWS_CERTIFICATE_PASSWORD` | Windows | Password for the `.pfx` |

When macOS secrets are omitted, unsigned macOS bundles are still published. The same
applies to Windows. Linux packages are always unsigned; distribute `SHA256SUMS.txt`
alongside them.

### Local helper scripts

| Script | Purpose |
| --- | --- |
| `scripts/import-apple-cert.sh` | Import a `.p12` into a temporary keychain (CI/local) |
| `scripts/sign-macos.sh` | `codesign` + optional `notarytool` staple for `.app`/`.dmg` |
| `scripts/sign-windows.sh` | Authenticode-sign `.exe`/`.msi`/installer artifacts |
| `scripts/release-checksums.sh` | Generate `SHA256SUMS.txt` for a directory of release files |

## macOS (manual)

Prerequisites: Apple Developer ID Application certificate, `notarytool` credentials.

```sh
scripts/build-macos.sh
export APPLE_SIGNING_IDENTITY="Developer ID Application: …"
# Optional notarization:
export APPLE_ID=… APPLE_APP_PASSWORD=… APPLE_TEAM_ID=…
scripts/sign-macos.sh
```

## Windows (manual)

Prerequisites: EV or standard code-signing certificate on a token or store.

```sh
scripts/build-windows.sh
export WINDOWS_CERTIFICATE_PASSWORD=…
scripts/sign-windows.sh /path/to/certificate.pfx
```

## Linux

Debian/RPM/AppImage builds are unsigned (AppImage via `scripts/build-appimage.sh` /
release workflow). Distribute checksums (SHA-256) alongside
release artifacts:

```sh
scripts/release-checksums.sh path/to/release-artifacts
```
