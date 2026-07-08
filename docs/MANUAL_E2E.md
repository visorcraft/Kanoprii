# Manual End-to-End Checklist

Automated WebDriver smoke tests cover launch, open-via-path, and rotate/dirty
state (`scripts/e2e-test.sh` / `npm run test:e2e`). Run this broader checklist
before a release tag.

## v0.5 cross-feature matrix

- [ ] Continuous scroll + tabs - open two PDFs, switch one tab to continuous
      scroll, scroll mid-document, switch tabs and back: view mode, scroll
      position, and current page survive the round-trip; single-page tab is
      unaffected
- [ ] OCR select/highlight - Document → Make Searchable (OCR) on a scanned PDF,
      then select OCR'd text in the viewer, copy it, and Annotations →
      Highlight Selection; Find locates the OCR'd words
- [ ] Redact + re-OCR - add redaction boxes over text, Document → Apply
      Redactions… with **Restore searchable text (OCR)** checked: covered text
      gone from Find/copy, remaining text searchable again, page count and size
      unchanged; undo restores the pre-apply document until save
- [ ] Bates - Document → Bates Numbering… (prefix + start + digits + position)
      across a page range; labels render at every page in the range and survive
      save/reopen; zoom keeps them aligned
- [ ] Edit text + undo - Annotations → Edit Text, click a text run, replace it
      (whiteout covers cleanly at 100% and 200% zoom), then undo: original text
      returns and dirty state tracks correctly
- [ ] Dirty-tab quit (multitab scenario 7) - two tabs with unsaved edits, quit
      the app: prompts appear sequentially with the matching tab focused for
      each; Cancel on the second prompt aborts quit with both documents intact

## Open & save
- [ ] Open PDF via path entry and via built-in browser
- [ ] Recently Opened list updates
- [ ] Edit → dirty indicator → Save commits working copy
- [ ] Save As writes a new file; unsaved prompt on close/open/quit

## View & edit
- [ ] Page navigation (toolbar, thumbnails, keyboard, wheel at scroll edges, bookmarks panel)
- [ ] Find text (Ctrl/Cmd+F) - search, next/previous, highlight on page
- [ ] Zoom 25%–400% with aligned highlights/notes/drawings
- [ ] Delete / duplicate / dup. before / dup. odd+even / dup. odd+even before / dup. odd+even to start+end / dup. range (before+to start+to end) / dup. all / dup. to end / rotate / rot. range (CW/CCW/180/reset) / rot. odd+even (CW/CCW/180/reset) / rotate all 180° / CCW / reset rot. / rotate all / all CCW / reverse / rev. range / rev. odd+even / odd→start / even→start / odd→end / even→end / blank before+after / blank pages / blank between / blank before+after odd+even / move up+down / move range (to start+end) / keep range / keep odd+even / del odd+even / parity range tools (global odd/even + local parity + in-range mod-3…mod-6 + doc-wide mod-3…mod-6 + half/third-range + sort desc) / swap / replace / interleave / prepend / odd-even split / split at / split N / sort ↑↓ / odd+even sort ↑↓ / rot sort ↑↓ / odd+even rot sort ↑↓ / delete range / delete nth / reorder / insert / image page / merge / extract / extract odd+even / export page PDF / export pages PDF / export odd+even pages PDF+image (PNG/JPEG/WebP/BMP/TIFF/GIF/PPM/TGA/ICO) / split
- [ ] Export PNG/JPEG/WebP/BMP/TIFF/GIF/PPM/TGA/ICO, page dimensions, page numbers / header / footer / border / watermark (incl. odd+even apply), page size presets, expand+shrink margins (incl. odd+even), crop range / crop odd+even, crop (single/all/clear all/odd+even), flatten annotations, flatten all/odd/even
- [ ] Bookmark all / odd / even pages; add / rename / remove / clear all bookmarks; page size odd+even
- [ ] Metadata modal - edit, Clear all, Apply marks dirty; Save persists

## Annotations
- [ ] Highlight add (H) and remove
- [ ] Sticky note add (N) and remove
- [ ] Freehand draw add (D) and remove
- [ ] Shape outlines add (S - rect/ellipse/line) and remove
- [ ] Text/image stamps add (T) and remove
- [ ] Redaction boxes add (X) and remove
- [ ] Page image insert (I) - path entry, two-click placement, re-render shows image
- [ ] Form fields (F) - list fields, apply values, add text/checkbox/choice/radio fields

## Security & signatures
- [ ] Digital sign (Ctrl/Cmd+Shift+U) with PKCS#12; Signatures panel verify status
- [ ] In-PDF page text (E) and vector rectangles (G); Edits modal

## Export
- [ ] Markdown toggle (Ctrl/Cmd+Shift+M); sibling `.md` auto-save; Save As… custom path; overwrite conflict prompt
- [ ] Summarize (Ctrl/Cmd+Shift+E); sibling `.summary.md`
- [ ] Scanned/no-text page saves PNG + OCR text in `<name>_assets/`; sparse/complex pages get page-render OCR supplement; embedded images extracted + OCR'd on save (needs Tesseract)
- [ ] Export image (Ctrl/Cmd+Shift+B) - PNG or JPEG, current page, range, or all pages
- [ ] Optimize, password-protect export, decrypt to `_decrypted.pdf`, and print
- [ ] Open an encrypted `_protected.pdf` with password prompt

## Platforms
- [ ] Linux Wayland - Open PDF via path or Browse… (no **Choose file…** unless `KANOPRII_NATIVE_DIALOGS=1`); open/save does not hang
- [ ] Linux Wayland - launch + page render without Gdk DMABUF protocol error (multi-GPU stack)
- [ ] Linux X11 - native **Choose file…** on Open/Save when offered
- [ ] macOS and Windows smoke pass

## Single-instance / Open With
- [ ] File manager double-click on PDF while app is running opens it in a new tab and focuses the window
- [ ] CLI `kanoprii a.pdf` while app is running opens `a.pdf` in a new tab
- [ ] Second CLI `kanoprii b.pdf` opens `b.pdf` in another tab (no second process)
- [ ] Re-opening an already-open file via CLI just focuses its tab
- [ ] First launch with a path arg (`kanoprii a.pdf` with no running instance) opens the file on startup
