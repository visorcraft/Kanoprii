# Credits and Attribution

## Copyright

Kanoprii is © VisorCraft LLC and contributors, distributed under the
[GNU General Public License v3.0](LICENSE).

## Runtime dependencies

Kanoprii links against or bundles the following components at execution
time. Packaged builds ship PDFium; Linux desktop builds use the system
WebKitGTK and GTK stacks. Direct printing on Linux and macOS uses CUPS.
Tesseract is optional and invoked only when the user enables scan OCR for
Markdown export.

| Component | License | Project |
| --------- | ------- | ------- |
| PDFium | BSD-3-Clause | https://pdfium.googlesource.com/pdfium/ |
| WebKitGTK | LGPL-2.1+ | https://webkitgtk.org/ |
| GTK 3 | LGPL-2.1+ | https://www.gtk.org/ |
| CUPS | Apache-2.0 | https://openprinting.github.io/cups/ |
| Tesseract OCR (optional) | Apache-2.0 | https://github.com/tesseract-ocr/tesseract |

Full license texts for these runtimes are bundled under [`LICENSES/`](LICENSES/)
and viewable in-app under **Help → Licenses**, **Runtime components** tab.

## Rust crate dependencies

The machine-generated transitive supplement - every Rust crate, its exact
version, and the full text of every distinct license - lives in
[`docs/credits-third-party.md`](docs/credits-third-party.md).
Regenerate it via `scripts/generate-credits.sh` (which runs `cargo-about`
over `src-tauri/Cargo.lock`).

### Direct Rust dependencies

These are the crates declared in `src-tauri/Cargo.toml` that are built into
release binaries (the optional `wdio` feature used only for E2E testing is
excluded).

| Crate | Version | License | Project |
| ----- | ------- | ------- | ------- |
| `tauri` | 2.11.3 | MIT OR Apache-2.0 | [tauri-apps/tauri](https://github.com/tauri-apps/tauri) |
| `tauri-build` | 2.6.3 | MIT OR Apache-2.0 | [tauri-apps/tauri](https://github.com/tauri-apps/tauri) |
| `tauri-plugin-dialog` | 2.7.1 | MIT OR Apache-2.0 | [tauri-apps/plugins-workspace](https://github.com/tauri-apps/plugins-workspace) |
| `tauri-plugin-process` | 2.3.1 | MIT OR Apache-2.0 | [tauri-apps/plugins-workspace](https://github.com/tauri-apps/plugins-workspace) |
| `tauri-plugin-single-instance` | 2.4.2 | MIT OR Apache-2.0 | [tauri-apps/plugins-workspace](https://github.com/tauri-apps/plugins-workspace) |
| `tauri-plugin-updater` | 2.10.1 | MIT OR Apache-2.0 | [tauri-apps/plugins-workspace](https://github.com/tauri-apps/plugins-workspace) |
| `pdfium-render` | 0.9.2 | MIT OR Apache-2.0 | [ajrcarey/pdfium-render](https://github.com/ajrcarey/pdfium-render) |
| `lopdf` | 0.42.0 | MIT | [J-F-Liu/lopdf](https://github.com/J-F-Liu/lopdf) |
| `image` | 0.25.10 | MIT OR Apache-2.0 | [image-rs/image](https://github.com/image-rs/image) |
| `fax` | 0.2.7 | MIT | [pdf-rs/fax](https://github.com/pdf-rs/fax) |
| `underskrift` | 0.1.4 | BSD-2-Clause | [kushaldas/underskrift](https://github.com/kushaldas/underskrift) |
| `serde` | 1.0.228 | MIT OR Apache-2.0 | [serde-rs/serde](https://github.com/serde-rs/serde) |
| `serde_json` | 1.0.150 | MIT OR Apache-2.0 | [serde-rs/json](https://github.com/serde-rs/json) |
| `tokio` | 1.52.3 | MIT | [tokio-rs/tokio](https://github.com/tokio-rs/tokio) |
| `ureq` | 2.12.1 | MIT OR Apache-2.0 | [algesten/ureq](https://github.com/algesten/ureq) |
| `sha2` | 0.10.9 | MIT OR Apache-2.0 | [RustCrypto/hashes](https://github.com/RustCrypto/hashes) |
| `ttf-parser` | 0.25.1 | MIT OR Apache-2.0 | [harfbuzz/ttf-parser](https://github.com/harfbuzz/ttf-parser) |
| `printers` | 2.3.0 | MIT | [talesluna/rust-printers](https://github.com/talesluna/rust-printers) |
| `openssl-sys` (Windows) | 0.9.117 | MIT | [rust-openssl/rust-openssl](https://github.com/rust-openssl/rust-openssl) |
| `windows` (Windows) | 0.62.2 | MIT OR Apache-2.0 | [microsoft/windows-rs](https://github.com/microsoft/windows-rs) |

## Frontend (npm) dependencies

The React shell ships a small set of direct npm packages from root
`package.json` `dependencies`. Dev-only WebdriverIO packages under `e2e/`
are omitted because they are not bundled into release artifacts.

| Package | Version | License | Project |
| ------- | ------- | ------- | ------- |
| `@tauri-apps/api` | 2.11.1 | MIT OR Apache-2.0 | [tauri-apps/tauri](https://github.com/tauri-apps/tauri) |
| `@tauri-apps/plugin-dialog` | 2.7.1 | MIT OR Apache-2.0 | [tauri-apps/plugins-workspace](https://github.com/tauri-apps/plugins-workspace) |
| `@tauri-apps/plugin-process` | 2.3.1 | MIT OR Apache-2.0 | [tauri-apps/plugins-workspace](https://github.com/tauri-apps/plugins-workspace) |
| `@tauri-apps/plugin-updater` | 2.10.1 | MIT OR Apache-2.0 | [tauri-apps/plugins-workspace](https://github.com/tauri-apps/plugins-workspace) |
| `react` | 19.2.7 | MIT | [react](https://react.dev/) |
| `react-dom` | 19.2.7 | MIT | [react](https://react.dev/) |

Regenerate `docs/credits-npm.json` and the npm license appendix via
`scripts/generate-credits.sh` after any npm dependency change.
