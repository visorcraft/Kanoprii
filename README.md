<p align="center">
  <img src="src/assets/kanoprii.png" alt="Kanoprii logo" width="200" />
</p>

<h1 align="center">Kanoprii</h1>

<p align="center">
  <a href="https://github.com/visorcraft/Kanoprii/releases/latest"><img src="https://img.shields.io/github/v/release/visorcraft/Kanoprii?sort=semver" alt="Latest release" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-GPL--3.0--only-blue.svg" alt="License: GPL-3.0-only" /></a>
  <img src="https://img.shields.io/badge/built%20with-Rust-000000?logo=rust&amp;logoColor=white" alt="Built with Rust" />
  <img src="https://img.shields.io/badge/Tauri%202-FFC131?logo=tauri&amp;logoColor=white" alt="Tauri 2" />
  <img src="https://img.shields.io/badge/platform-Linux%20%7C%20macOS%20%7C%20Windows-333333?logo=linux&amp;logoColor=white" alt="Platform: Linux, macOS, Windows" />
</p>

<p align="center">
  <b>The friendly, fast, cross-platform PDF editor.</b><br />
  Open a document, rearrange pages, annotate, sign, and export to Markdown - all in one lightweight native app.
</p>

---

## Screenshots

<p align="center">
  <img src="docs/screenshots/welcome-screen.png" alt="Kanoprii welcome screen with menu bar and click-to-open prompt" width="720" />
</p>

<p align="center">
  <i>Welcome screen - custom title bar, menu bar, and click-to-open splash before a document is loaded.</i>
</p>

---

## What is Kanoprii?

Kanoprii is a desktop PDF editor for the everyday workflow: **view, reorganize,
annotate, sign, and export** - without firing up a heavyweight suite or uploading
your files to someone else's server. It's a small, snappy native app that gets out
of your way.

Three things we care about:

- **Native & cross-platform** - one lightweight [Tauri 2](https://v2.tauri.app/) app that feels at home on **Linux**, **macOS**, and **Windows**.
- **Non-destructive** - every edit lands in a working copy, so your original file stays untouched until you choose to save. Undo/redo has your back (50 steps).
- **Offline & private** - no cloud, no accounts, no telemetry. Your documents never leave your machine.

### What it covers today

**View & navigate**
- **Menu bar** (File, Edit, Pages, Document, Annotate, Security, View, Help) plus a slim quick-access row - all page tools live in menus; **Ctrl/Cmd+Shift+P** opens a searchable command palette
- Smooth viewer with **25%–400%** zoom; thumbnail sidebar matches viewer height
- Page navigation via menus, quick bar, thumbnails, keyboard, mouse wheel at scroll edges, and a clickable **Bookmarks** outline
- **Find text** across the document with match highlighting (Ctrl/Cmd+F); **selectable text layer** with native copy and **Highlight Selection**
- **Continuous scroll** mode (View → Continuous scroll) with virtualized page rendering
- **Document tabs** - multiple PDF, Markdown, and HTML documents open at once (Ctrl+Tab / Ctrl+W)
- Open PDF, Markdown (`.md`), or HTML (`.html`) via **File → Open Document** (Ctrl/Cmd+O), in-app path entry, **Recently Opened**, built-in browser, or **native open/save dialogs**

**Organize pages**
- **Delete**, **duplicate**, **rotate** (90° steps), and **drag-and-drop reorder**
- **Insert** pages from another PDF (range + position; merges form fields, dedups fonts)
- **Merge** another PDF by appending its pages to the end (page range supported)
- **Split** into multiple files by page range
- **Extract** a page range into a new PDF without changing the open document
- **Reverse** page order, **rotate all** pages, insert a **blank page**, or **delete a page range**
- **Export PNG/JPEG/WebP/BMP/TIFF/GIF/PPM/TGA/ICO** or **export current page / each page as PDF**
- **Rotate 180°**, **rotate all CCW**, **move page to first/last/up/down**, **swap pages**, **blank before/after**
- **Insert image as new page**, **page header/footer** text, **duplicate page range** or **duplicate all**
- **Replace** current page, **interleave** or **prepend** another PDF, **split odd/even** or **split every N**, **set page size** (Letter/A4/Legal)
- **Rotate/move/keep/reverse page range**, **reset rotation range**, **crop range**, **crop/expand/shrink odd/even**, **clear crop odd/even**, **insert blank pages**, **blank between/before/after odd/even**, **duplicate range to start/end/before**, **duplicate page/odd/even** (append, before, to start, or to end), **rotate odd/even** (CW/CCW/180/reset), **rotate all 180°**, **keep/delete odd/even**, **flatten odd/even**, **sort by rotation/size** (all or odd/even parity), **delete every Nth**, **move range to start/end**, **reverse odd/even**, **move odd/even to start/end**, **extract/export odd/even** (PDF or PNG/JPEG/WebP/BMP/TIFF/GIF/PPM/TGA/ICO), **parity tools** (1766 commands: in-range global/local/mod-3…mod-6/half/third-range + sort desc + document-wide mod-3…mod-6 via Parity Range modal), **split at page**, **shrink margins**, **bookmark/size odd/even**
- **Page border**, **expand margins**, **bookmark all pages**, **flatten all**, **sort by page size**, **clear metadata**
- **Add/rename/remove/clear bookmarks**, **page dimensions**, **page numbers**, **headers/footers/borders/watermarks** (all or odd/even parity), **crop** (single/all/clear), **flatten** annotations
- View and edit document **metadata** (title, author, subject, keywords, creator, producer)

**Annotate & mark up**
- **Highlights** (`H`), **sticky notes** (`N`), **freehand ink** (`D`), **shapes** (`S` - rectangle/ellipse/line)
- **Stamps** (`T` - APPROVED, DRAFT, CONFIDENTIAL, REVIEWED, plus image stamps) and **redaction** boxes (`X`)
- **In-PDF text blocks** (`E`), **Edit text** (click existing text runs), and **vector rectangles** (`G`), embedded **images** (`I`) - all persisted in the PDF
- **Annotations panel** (View) - document-wide list with navigate and delete
- **Apply redactions** - burn in redaction boxes (optional OCR restore); **Bates numbering**; **Make Searchable (OCR)** for scanned pages

**Forms & signatures**
- **Interactive forms** (`F`) - list, fill, and create text / checkbox / choice / radio fields
- **PAdES digital signatures** - sign with a PKCS#12 certificate (`.p12`/`.pfx`); list and verify in the Signatures panel
- **Password protect** - export an encrypted copy; **decrypt** to `_decrypted.pdf`; open encrypted PDFs with a prompt

**Convert & export**
- **Markdown / HTML → PDF** - Markdown opens in Markdown view; HTML opens in a sandboxed Webpage view with local CSS and assets. Switching to PDF View renders a collision-safe sibling PDF without overwriting an existing file.
- **PDF → Markdown** - tagged-PDF structure when available (headings, lists, tables with THead/TBody, links, TOC, captions, code, notes, inline emphasis); otherwise PDFium heuristic layout with column reading order, paragraph merge, header/footer stripping, and link/URL autodetection; on save, scanned/sparse pages, embedded images (JPEG/PNG/Gray/CMYK/Indexed/JPX, CCITT Fax scans, RunLength 1-bit), and painted Form XObjects (vector charts rasterized via PDFium, including nested form content) land in `<md-stem>_assets/` with optional Tesseract OCR; auto-save sibling `.md` or **Save As…** for a custom path
- **Summarize** - extractive overview, key points, and extracted headings/emails/URLs/dates
- **Optimize** - strip metadata, recompress images, prune unused objects, compress streams
- **Export PNG/JPEG/WebP/BMP/TIFF/GIF/PPM/TGA/ICO** - save rendered pages as image files (Ctrl/Cmd+Shift+B)
- **Print** with Kanoprii's preview dialog (printer, range, paper, scaling, margins, copies, duplex, save-as-PDF) or hand off to the system print dialog; print-only hidden watermarks/layers can be included or suppressed
- **Help → Check for Updates…** (AppImage / macOS / Windows; signed `latest.json` when release artifacts are published)

---

## Setup (build from source)

### Prerequisites

- [Node.js](https://nodejs.org/) 24+
- [Rust](https://rustup.rs/) (edition 2021)
- **Linux:** GTK/WebKit dev packages plus `libcups2-dev` (see [`.github/workflows/ci.yml`](.github/workflows/ci.yml) for the exact apt list)
- **Linux (optional):** `mold` linker and `sccache` for faster builds (configured in `.cargo/config.toml`)

### Fetch PDFium (required for rendering)

Rendering uses [`pdfium-render`](https://crates.io/crates/pdfium-render) against a
standard PDFium build (the C `FPDF_*` API). Fetch the prebuilt library first:

```sh
scripts/fetch-pdfium.sh
```

This installs into `src-tauri/vendor/pdfium/` (gitignored). The app also resolves
PDFium via `PDFIUM_LIB_PATH`, next to the executable, the bundled resource path,
then the system library.

> **Note:** distro packages such as `libdeepin-pdfium` expose a *different* C++ API and are **not** compatible.

### Tesseract OCR (optional, for scans)

Markdown export can OCR scanned pages, sparse-text pages, and embedded/form images when [Tesseract](https://github.com/tesseract-ocr/tesseract) is installed.

| Platform | Typical install |
| --- | --- |
| Linux | `tesseract` + `tesseract-data-eng` (or your distro’s language packs) |
| macOS | `brew install tesseract` |
| Windows | [UB Mannheim builds](https://github.com/UB-Mannheim/tesseract/wiki) |

**Markdown view (Ctrl/Cmd+Shift+M)** runs the full save pipeline: page PNGs and image assets land in `<md-stem>_assets/`, with OCR blocks appended where Tesseract is available. Without Tesseract you still get PDFium text and image assets; scanned pages show a plain-language notice in the header/toast. On first launch (and when opening Markdown without Tesseract), the app offers an optional install guide with a **Do not remind me again** checkbox; **Help → Install Tesseract** appears only while OCR is not detected.

The `convert_pdf_to_markdown` API is text/heuristics only (no assets, no OCR) - the UI does not use it.

### Run in development

```sh
npm install
npm run tauri dev
```

> Always use the **Tauri CLI** for dev and release builds - a plain
> `cargo build --release` produces a dev-mode binary that expects the Vite dev
> server.

---

## Install

Grab a prebuilt package from the [**Releases**](https://github.com/visorcraft/Kanoprii/releases/latest)
page, or build your own with the helpers under [`scripts/`](scripts/):

| Target | Command |
| --- | --- |
| Linux - `.deb` / `.rpm` | `scripts/build-linux-packages.sh` |
| Linux - AppImage | `scripts/build-appimage.sh` (prefetches linuxdeploy; `NO_STRIP=1` default on glibc 2.38+) |
| macOS - `.app` / `.dmg` | `scripts/build-macos.sh` |
| Windows - `.msi` / `.exe` | `scripts/build-windows.sh` |
| Any - standalone binary | `npx tauri build --no-bundle` → `src-tauri/target/release/kanoprii` |

Run the GitHub Actions [release workflow](.github/workflows/release.yml) manually
(**Actions → Release → Run workflow**) to build and publish `.deb`, `.rpm`,
AppImage, macOS, and Windows artifacts (unsigned by default; optional
macOS/Windows **package** signing via repository secrets - see
[`docs/SIGNING.md`](docs/SIGNING.md)).

---

## Tweak

### Keyboard shortcuts

| Shortcut | Action |
| --- | --- |
| Ctrl/Cmd+O | Open PDF (File menu) |
| Ctrl/Cmd+Shift+P | Command palette (search all actions) |
| Ctrl/Cmd+S / Ctrl/Cmd+Shift+S | Save / Save As |
| Ctrl/Cmd+W | Close PDF |
| Ctrl/Cmd+Z / Ctrl/Cmd+Y | Undo / Redo |
| Ctrl/Cmd+R | Rotate page |
| Ctrl/Cmd+P | Print |
| Ctrl/Cmd+Shift+M | PDF ↔ Markdown |
| Ctrl/Cmd+Shift+O | Optimize |
| Ctrl/Cmd+Shift+E | Summarize |
| Ctrl/Cmd+Shift+U | Digital sign |
| Ctrl/Cmd+F | Find text in PDF |
| Ctrl/Cmd+Shift+I / Ctrl/Cmd+Shift+G / Ctrl/Cmd+Shift+J / Ctrl/Cmd+Shift+K / Ctrl/Cmd+Shift+B | Insert / Merge / Extract / Split / Export image |
| Ctrl/Cmd+Shift+N / Ctrl/Cmd+Shift+Y | Insert blank page / Reverse page order |
| Ctrl/Cmd+0, Ctrl/Cmd +/− | Reset / change zoom |
| H N D S T X E G I F | Highlight / note / draw / shape / stamp / redact / page text / vector / image / forms |
| Delete · Escape | Delete page (confirm) · exit tool or close modal |

### Linux (Wayland & GPU)

- **File dialogs:** Open/Save default to in-app path entry and the built-in PDF browser on **Linux Wayland** - the XDG desktop-portal picker can hang WebKitGTK on some stacks. macOS, Windows, and Linux X11 use native **Choose file…** buttons by default. Set `KANOPRII_NATIVE_DIALOGS=1` to opt in on Wayland; `KANOPRII_DISABLE_NATIVE_DIALOGS=1` forces in-app paths everywhere.
- **DMABUF renderer:** At startup the app sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` when unset, avoiding `Gdk Error 71 (Protocol error)` crashes on some multi-GPU Wayland setups. GPU compositing stays on; only zero-copy presentation is disabled. Set your own value before launch to override.

### Environment variables

| Variable | Purpose |
| --- | --- |
| `PDFIUM_LIB_PATH` | Override path to the PDFium shared library |
| `KANOPRII_OCR_LANG` | Tesseract language(s), e.g. `eng`, `deu`, or `eng+deu` (default `eng`) |
| `TESSERACT_CMD` | Path to the `tesseract` executable when not on `PATH` |
| `KANOPRII_TESSDATA_PREFIX` / `TESSDATA_PREFIX` | Directory containing `.traineddata` files |
| `KANOPRII_OCR_PSM` | Page segmentation mode 0–13 (default `1`, auto page + OSD) |
| `KANOPRII_NATIVE_DIALOGS` | `1` = enable native file dialogs on Linux Wayland |
| `KANOPRII_DISABLE_NATIVE_DIALOGS` | `1` = in-app path entry only (all platforms) |
| `WEBKIT_DISABLE_DMABUF_RENDERER` | `1` = disable WebKitGTK DMABUF (set automatically on Linux when unset) |
| `KANOPRII_TEST_PDF` | PDF path for the ignored `render_real_pdf_smoke` integration test |

---

## Contribute

Contributions are welcome - issues, fixes, and features alike.

**Quick rules:**

- Match the existing style, and keep commits focused.
- Run the quality gates before you push: `scripts/smoke-test.sh` (Rust unit tests, `clippy`, `fmt`, `tsc`, `eslint`, `npm audit`) and, on Linux, `npm run test:e2e`.
- **Help → Licenses** shows bundled GPL, third-party (Rust + shipped npm), acknowledgment, and runtime license texts offline - searchable with line numbers, wrap, and copy.
- **Help → Credits** lists every Cargo crate, shipped npm package, and runtime component with filter, license badges, and project links.
- Regenerate license supplements with `scripts/generate-credits.sh` after Rust or npm dependency changes.
- Kanoprii is **GPL v3** - derivative works must stay open source under compatible terms.

---

## Licence

Kanoprii is licensed under the [GNU General Public License v3.0](LICENSE). Copyright © VisorCraft LLC.
