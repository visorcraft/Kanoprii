#!/usr/bin/env python3
"""Generate batch 46–55: 44 half-range extended + 56 mod-4-in-range commands."""

from __future__ import annotations

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
INC4 = ROOT / "src-tauri" / "src" / "parity_batch4_generated.inc.rs"
INC1 = ROOT / "src-tauri" / "src" / "parity_batch_generated.inc.rs"
MAIN = ROOT / "src-tauri" / "src" / "main.rs"
UI_JSON = ROOT / "src" / "parity_batch_commands.json"

HALF_EXTENDED = [
    ("duplicate_pages_in_range_before", "dup_before"),
    ("duplicate_pages_in_range_to_start", "dup_to_start"),
    ("duplicate_pages_in_range_to_end", "dup_to_end"),
    ("expand_pages_in_range", "expand"),
    ("shrink_pages_in_range", "shrink"),
    ("clear_crop_pages_in_range", "clear_crop"),
    ("insert_blank_before_pages_in_range", "blank_before"),
    ("insert_blank_after_pages_in_range", "blank_after"),
    ("bookmark_pages_in_range", "bookmark"),
    ("set_page_size_pages_in_range", "page_size"),
    ("add_page_header_pages_in_range", "header"),
    ("add_page_footer_pages_in_range", "footer"),
    ("add_page_border_pages_in_range", "border"),
    ("export_pages_in_range_webp", "export_webp"),
    ("export_pages_in_range_bmp", "export_bmp"),
    ("export_pages_in_range_tiff", "export_tiff"),
    ("export_pages_in_range_gif", "export_gif"),
    ("export_pages_in_range_ppm", "export_ppm"),
    ("export_pages_in_range_tga", "export_tga"),
    ("export_pages_in_range_ico", "export_ico"),
    ("sort_pages_in_range_by_rotation", "sort_rot"),
    ("sort_pages_in_range_by_size", "sort_size"),
]

MOD4_TRIPLET = [
    ("rotate_pages_in_range_mod4", "rotate_cw"),
    ("rotate_pages_in_range_mod4_ccw", "rotate_ccw"),
    ("rotate_180_pages_in_range_mod4", "rotate_180"),
    ("reset_rotation_pages_in_range_mod4", "reset_rot"),
    ("delete_pages_in_range_mod4", "delete"),
    ("keep_pages_in_range_mod4", "keep"),
    ("duplicate_pages_in_range_mod4", "dup_append"),
    ("flatten_pages_in_range_mod4", "flatten"),
    ("reverse_pages_in_range_mod4", "reverse"),
    ("crop_pages_in_range_mod4", "crop"),
    ("extract_pages_in_range_mod4", "extract"),
    ("export_pages_in_range_mod4_as_pdf", "export_pdf"),
    ("export_pages_in_range_mod4_png", "export_png"),
    ("export_pages_in_range_mod4_jpeg", "export_jpeg"),
]

HALF_SUPPLEMENTAL = [
    "parity_half_dup_before",
    "parity_half_dup_to_start",
    "parity_half_dup_to_end",
    "parity_half_expand",
    "parity_half_shrink",
    "parity_half_clear_crop",
    "parity_half_blank",
    "parity_half_bookmark",
    "parity_half_page_size",
    "parity_half_header",
    "parity_half_footer",
    "parity_half_border",
    "parity_half_sort_rotation",
    "parity_half_sort_size",
]

MOD4_HELPERS = [
    "parity_mod4_indices",
    "parity_mod4_rotate",
    "parity_mod4_reset_rotation",
    "parity_mod4_delete",
    "parity_mod4_keep",
    "parity_mod4_dup_append",
    "parity_mod4_flatten",
    "parity_mod4_reverse",
    "parity_mod4_crop",
    "parity_mod4_extract",
    "parity_mod4_export_pdf",
    "parity_mod4_export_rendered",
]


def mod4_name(base: str, remainder: int) -> str:
    return base.replace("_pages", f"_mod4_{remainder}_pages")


def half_name(base: str, first_half: bool) -> str:
    label = "first_half" if first_half else "second_half"
    return base.replace("_pages", f"_{label}_pages", 1)


def batch1_helper_chunk() -> str:
    body = INC1.read_text()
    start = body.index("fn parity_batch_indices")
    end = body.index("fn parity_batch_export_ico_doc")
    return body[start:end]


def half_supplemental_helpers() -> str:
    chunk = batch1_helper_chunk()
    chunk = (
        "fn parity_half_match(page_index: u32, start_page: u32, end_page: u32, first_half: bool) -> bool {\n"
        "    if page_index < start_page || page_index > end_page {\n"
        "        return false;\n"
        "    }\n"
        "    let mid = start_page + (end_page - start_page) / 2;\n"
        "    if first_half {\n"
        "        page_index <= mid\n"
        "    } else {\n"
        "        page_index > mid\n"
        "    }\n"
        "}\n\n" + chunk
    )
    chunk = chunk.replace("parity_batch_match", "parity_half_match")
    chunk = chunk.replace("parity_batch_", "parity_half_")
    chunk = re.sub(r"odd: bool, local: bool", "first_half: bool", chunk)
    chunk = chunk.replace("keep_odd: bool, local: bool", "keep_first_half: bool")
    chunk = chunk.replace(", odd, local)", ", first_half)")
    chunk = chunk.replace(
        "parity_half_match(idx, start_page, end_page, keep_odd, local)",
        "parity_half_match(idx, start_page, end_page, keep_first_half)",
    )
    end_lookahead = r"(?=\n(?:#\[allow|fn parity_half_)|\Z)"
    kept: list[str] = []
    for name in HALF_SUPPLEMENTAL:
        m = re.search(
            rf"(#\[allow\(clippy::too_many_arguments\)\]\n)?fn {name}\(.*?{end_lookahead}",
            chunk,
            flags=re.DOTALL,
        )
        if m:
            kept.append(m.group(0).rstrip())
    return "\n\n".join(kept) + ("\n\n" if kept else "")


def mod4_rust_helpers() -> str:
    chunk = batch1_helper_chunk()
    chunk = (
        "fn parity_mod4_match(page_index: u32, start_page: u32, end_page: u32, remainder: u32) -> bool {\n"
        "    if page_index < start_page || page_index > end_page {\n"
        "        return false;\n"
        "    }\n"
        "    (page_index - start_page) % 4 == remainder\n"
        "}\n\n" + chunk
    )
    chunk = chunk.replace("parity_batch_match", "parity_mod4_match")
    chunk = chunk.replace("parity_batch_", "parity_mod4_")
    chunk = re.sub(r"odd: bool, local: bool", "remainder: u32", chunk)
    chunk = chunk.replace("keep_remainder: u32", "remainder: u32")
    chunk = chunk.replace(", odd, local)", ", remainder)")
    chunk = chunk.replace(
        "parity_mod4_match(idx, start_page, end_page, keep_odd, local)",
        "parity_mod4_match(idx, start_page, end_page, remainder)",
    )
    chunk = re.sub(
        r"#\[allow\(clippy::manual_is_multiple_of\)\]\nfn parity_mod4_move_seg.*?^}\n",
        "",
        chunk,
        flags=re.MULTILINE | re.DOTALL,
    )
    unused = [
        name
        for name in re.findall(r"fn (parity_mod4_\w+)", chunk)
        if name not in MOD4_HELPERS and name != "parity_mod4_match"
    ]
    for name in unused:
        chunk = re.sub(
            rf"(#\[allow\(clippy::too_many_arguments\)\]\n)?fn {name}\(.*?(?=\n(?:#\[allow|fn parity_mod4_))",
            "",
            chunk,
            flags=re.DOTALL,
        )
    end_lookahead = r"(?=\n(?:#\[allow|fn parity_mod4_)|\Z)"
    preamble = (
        "fn parity_mod4_match(page_index: u32, start_page: u32, end_page: u32, remainder: u32) -> bool {\n"
        "    if page_index < start_page || page_index > end_page {\n"
        "        return false;\n"
        "    }\n"
        "    (page_index - start_page) % 4 == remainder\n"
        "}\n"
    )
    kept: list[str] = []
    for name in MOD4_HELPERS:
        m = re.search(
            rf"(#\[allow\(clippy::too_many_arguments\)\]\n)?fn {name}\(.*?{end_lookahead}",
            chunk,
            flags=re.DOTALL,
        )
        if m:
            kept.append(m.group(0).rstrip())
    body = "\n\n".join(kept)
    return preamble + ("\n\n" + body if body else "\n")


def command_sig(kind: str) -> tuple[str, str]:
    base = "path: String, start_page: u32, end_page: u32"
    ret = "Result<u32, String>"
    if kind == "extract":
        return f"{base}, output_path: String", "Result<String, String>"
    if kind.startswith("export"):
        return f"{base}, output_dir: String", "Result<Vec<String>, String>"
    if kind in ("crop", "expand", "shrink"):
        return f"{base}, margin_top: f64, margin_right: f64, margin_bottom: f64, margin_left: f64", ret
    if kind in ("bookmark", "page_numbers"):
        return f"{base}, prefix: Option<String>", ret
    if kind in ("watermark", "header", "footer"):
        return f"{base}, text: String", ret
    if kind == "page_size":
        return f"{base}, preset: String", ret
    if kind == "border":
        return f"{base}, inset: f64", ret
    return base, ret


def half_impl_call(kind: str, first_half: bool) -> str:
    fh = "true" if first_half else "false"
    path = "&PathBuf::from(&path)"
    rng = f"{path}, start_page, end_page"
    pl = f"{rng}, {fh}"
    calls = {
        "dup_before": f"parity_half_dup_before({pl})",
        "dup_to_start": f"parity_half_dup_to_start({pl})",
        "dup_to_end": f"parity_half_dup_to_end({pl})",
        "expand": f"parity_half_expand({pl}, margin_top, margin_right, margin_bottom, margin_left)",
        "shrink": f"parity_half_shrink({pl}, margin_top, margin_right, margin_bottom, margin_left)",
        "clear_crop": f"parity_half_clear_crop({pl})",
        "blank_before": f"parity_half_blank({pl}, false)",
        "blank_after": f"parity_half_blank({pl}, true)",
        "bookmark": f"parity_half_bookmark({pl}, prefix)",
        "page_size": f"parity_half_page_size({pl}, &preset)",
        "header": f"parity_half_header({pl}, &text)",
        "footer": f"parity_half_footer({pl}, &text)",
        "border": f"parity_half_border({pl}, inset)",
        "export_webp": f'parity_half_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {fh}, "webp", render_page_webp)',
        "export_bmp": f'parity_half_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {fh}, "bmp", render_page_bmp)',
        "export_tiff": f'parity_half_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {fh}, "tiff", render_page_tiff)',
        "export_gif": f'parity_half_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {fh}, "gif", render_page_gif)',
        "export_ppm": f'parity_half_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {fh}, "ppm", render_page_ppm)',
        "export_tga": f'parity_half_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {fh}, "tga", render_page_tga)',
        "export_ico": f'parity_half_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {fh}, "ico", render_page_ico)',
        "sort_rot": f"parity_half_sort_rotation({pl}, false)",
        "sort_size": f"parity_half_sort_size({pl}, false)",
    }
    return calls[kind]


def mod4_impl_call(kind: str, remainder: int) -> str:
    r = str(remainder)
    path = "&PathBuf::from(&path)"
    rng = f"{path}, start_page, end_page"
    pl = f"{rng}, {r}"
    calls = {
        "rotate_cw": f"parity_mod4_rotate({pl}, 90)",
        "rotate_ccw": f"parity_mod4_rotate({pl}, -90)",
        "rotate_180": f"parity_mod4_rotate({pl}, 180)",
        "reset_rot": f"parity_mod4_reset_rotation({pl})",
        "delete": f"parity_mod4_delete({pl})",
        "keep": f"parity_mod4_keep({pl})",
        "dup_append": f"parity_mod4_dup_append({pl})",
        "flatten": f"parity_mod4_flatten({pl})",
        "reverse": f"parity_mod4_reverse({pl})",
        "crop": f"parity_mod4_crop({pl}, margin_top, margin_right, margin_bottom, margin_left)",
        "extract": f"parity_mod4_extract({path}, &PathBuf::from(&output_path), start_page, end_page, {r})",
        "export_pdf": f"parity_mod4_export_pdf({path}, &PathBuf::from(&output_dir), start_page, end_page, {r})",
        "export_png": f'parity_mod4_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {r}, "png", render_page_png)',
        "export_jpeg": f'parity_mod4_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {r}, "jpeg", render_page_jpeg)',
    }
    return calls[kind]


def gen_half_command(name: str, kind: str, first_half: bool) -> str:
    sig, ret = command_sig(kind)
    call = half_impl_call(kind, first_half)
    label = "first half" if first_half else "second half"
    return f"""
/// {label.capitalize()} of page range - {kind}
#[tauri::command]
fn {name}({sig}) -> {ret} {{
    {call}
}}
"""


def gen_mod4_command(name: str, kind: str, remainder: int) -> str:
    sig, ret = command_sig(kind)
    call = mod4_impl_call(kind, remainder)
    return f"""
/// Mod-4 remainder {remainder} in range - {kind}
#[tauri::command]
fn {name}({sig}) -> {ret} {{
    {call}
}}
"""


def gen_test(name: str, kind: str) -> str:
    setup = ""
    extra = ""
    if kind == "extract":
        setup = '        let output_path = tmp("extract_out.pdf");\n'
        extra = ", output_path.to_string_lossy().into_owned()"
    elif kind.startswith("export"):
        setup = '        let output_dir = tmp("export_dir");\n'
        extra = ", output_dir.to_string_lossy().into_owned()"
    elif kind in ("crop", "expand", "shrink"):
        extra = ", 0.0, 0.0, 0.0, 0.0"
    elif kind in ("bookmark", "page_numbers"):
        extra = ", None"
    elif kind in ("watermark", "header", "footer"):
        extra = ', "wm".to_string()'
    elif kind == "page_size":
        extra = ', "letter".to_string()'
    elif kind == "border":
        extra = ", 1.0"
    return f"""
    #[test]
    fn {name}_rejects_invalid_range() {{
        let path = save(&mut build_pdf(2), "{name}");
{setup}        let err = {name}(path.clone(), 5, 10{extra}).unwrap_err();
        assert!(err.contains("Invalid page range"));
        let _ = std::fs::remove_file(&path);
    }}
"""


def build_specs() -> list[tuple[str, str, str]]:
    specs: list[tuple[str, str, str]] = []
    for base, kind in HALF_EXTENDED:
        specs.append((half_name(base, True), kind, "half"))
        specs.append((half_name(base, False), kind, "half"))
    for base, kind in MOD4_TRIPLET:
        for rem in (0, 1, 2, 3):
            specs.append((mod4_name(base, rem), kind, "mod4"))
    return specs


def load_prior_command_names() -> list[str]:
    import importlib.util

    names: list[str] = []
    for script in ("gen-parity-batch.py", "gen-parity-batch2.py", "gen-parity-batch3.py"):
        path = ROOT / "scripts" / script
        spec = importlib.util.spec_from_file_location(script, path)
        mod = importlib.util.module_from_spec(spec)
        assert spec.loader is not None
        spec.loader.exec_module(mod)
        names.extend(s[0] for s in mod.build_specs())
    return names


def main() -> None:
    specs = build_specs()
    assert len(specs) == 100, len(specs)

    lines = [
        "// Auto-generated by scripts/gen-parity-batch4.py - do not edit.",
        half_supplemental_helpers(),
        mod4_rust_helpers(),
    ]
    tests = ["// Auto-generated parity batch4 tests"]
    handlers: list[str] = []

    for name, kind, cat in specs:
        if cat == "half":
            first_half = "_first_half_" in name
            lines.append(gen_half_command(name, kind, first_half))
        else:
            rem = int(re.search(r"_mod4_(\d)_", name).group(1))  # type: ignore[union-attr]
            lines.append(gen_mod4_command(name, kind, rem))
        tests.append(gen_test(name, kind))
        handlers.append(f"            {name},")

    INC4.write_text("\n".join(lines))
    print(f"Wrote {INC4} ({len(specs)} commands)")

    import sys

    sys.path.insert(0, str(ROOT / "scripts"))
    from parity_patch import patch_sources

    patch_sources("BATCH4", handlers, tests)

    merged = load_prior_command_names() + [s[0] for s in specs]
    UI_JSON.write_text(json.dumps(merged, indent=2))
    print(f"Wrote {UI_JSON} ({len(merged)} total commands)")


if __name__ == "__main__":
    main()
