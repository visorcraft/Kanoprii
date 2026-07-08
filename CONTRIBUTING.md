# Contributing to Kanoprii

Thanks for your interest in Kanoprii. Issues, bug fixes, and feature work are
welcome.

## Before you start

- Search [existing issues](https://github.com/visorcraft/Kanoprii/issues) so we
  do not duplicate work.
- For larger changes, open an issue first to agree on approach and scope.
- Kanoprii is **GPL v3**. Contributions you submit must be compatible with that
  licence.

## Development setup

See [README.md](README.md) for prerequisites, PDFium setup, and how to run the
app locally.

Quick start:

```sh
scripts/fetch-pdfium.sh
npm install
npm run tauri dev
```

Use the **Tauri CLI** for dev and release builds. A plain `cargo build --release`
produces a dev-mode binary that expects the Vite dev server.

## Quality gates

Run these before opening a pull request:

```sh
scripts/smoke-test.sh
```

CI runs the same gates on push/PR to `master` (see `.github/workflows/ci.yml`).

On Linux, also run:

```sh
npm run test:e2e
```

That script installs WebdriverIO deps under `e2e/` automatically; a normal
`npm install` at the repo root does not pull them in.

From `src-tauri/` the smoke script covers Rust unit tests, `clippy`, `fmt`, and
`npx tsc --noEmit`. Optional integration coverage with PDFium:

```sh
KANOPRII_TEST_PDF=/path/to.pdf cargo test render_real_pdf_smoke -- --ignored --nocapture
```

CI and release workflows are **manual only** - run them from **Actions** when
needed.

## Pull requests

- Keep commits focused; one logical change per PR when possible.
- Match existing code style and conventions in the touched files.
- Update user-facing docs when behaviour changes.
- Do not add unrelated refactors or drive-by edits.

## Questions

Open a [GitHub issue](https://github.com/visorcraft/Kanoprii/issues) or visit
[visorcraft.com](https://www.visorcraft.com).
