#!/usr/bin/env python3
"""Generate batch 26–35: 60 local-parity completions + 40 mod-3-in-range commands."""

from __future__ import annotations

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
INC2 = ROOT / "src-tauri" / "src" / "parity_batch2_generated.inc.rs"
INC1 = ROOT / "src-tauri" / "src" / "parity_batch_generated.inc.rs"
MAIN = ROOT / "src-tauri" / "src" / "main.rs"
UI_JSON = ROOT / "src" / "parity_batch_commands.json"

# 30 × 2 = 60 local-parity completions (reuse parity_batch_* with local=true)
LOCAL_EXTENDED = [
    ("reverse_range_local_pages", "reverse"),
    ("move_odd_range_local_pages_to_start", "move_odd_start"),
    ("move_even_range_local_pages_to_start", "move_even_start"),
    ("move_odd_range_local_pages_to_end", "move_odd_end"),
    ("move_even_range_local_pages_to_end", "move_even_end"),
    ("sort_range_local_pages_by_rotation", "sort_rot"),
    ("sort_range_local_pages_by_size", "sort_size"),
    ("duplicate_range_local_pages_to_start", "dup_to_start"),
    ("duplicate_range_local_pages_to_end", "dup_to_end"),
    ("crop_range_local_pages", "crop"),
    ("expand_range_local_pages", "expand"),
    ("shrink_range_local_pages", "shrink"),
    ("clear_crop_range_local_pages", "clear_crop"),
    ("insert_blank_before_range_local_pages", "blank_before"),
    ("insert_blank_after_range_local_pages", "blank_after"),
    ("bookmark_range_local_pages", "bookmark"),
    ("set_page_size_range_local_pages", "page_size"),
    ("extract_range_local_pages", "extract"),
    ("add_page_numbers_range_local_pages", "page_numbers"),
    ("add_text_watermark_range_local_pages", "watermark"),
    ("add_page_header_range_local_pages", "header"),
    ("add_page_footer_range_local_pages", "footer"),
    ("add_page_border_range_local_pages", "border"),
    ("export_range_local_pages_as_pdf", "export_pdf"),
    ("export_range_local_pages_png", "export_png"),
    ("export_range_local_pages_jpeg", "export_jpeg"),
    ("export_range_local_pages_webp", "export_webp"),
    ("export_range_local_pages_bmp", "export_bmp"),
    ("export_range_local_pages_tiff", "export_tiff"),
    ("export_range_local_pages_gif", "export_gif"),
    ("export_range_local_pages_ppm", "export_ppm"),
    ("export_range_local_pages_tga", "export_tga"),
]

# 12 × 3 + 4 = 40 mod-3-in-range commands
MOD3_TRIPLET = [
    ("rotate_pages_in_range_mod3", "rotate_cw"),
    ("rotate_pages_in_range_mod3_ccw", "rotate_ccw"),
    ("rotate_180_pages_in_range_mod3", "rotate_180"),
    ("reset_rotation_pages_in_range_mod3", "reset_rot"),
    ("delete_pages_in_range_mod3", "delete"),
    ("keep_pages_in_range_mod3", "keep"),
    ("duplicate_pages_in_range_mod3", "dup_append"),
    ("flatten_pages_in_range_mod3", "flatten"),
    ("reverse_pages_in_range_mod3", "reverse"),
    ("crop_pages_in_range_mod3", "crop"),
    ("extract_pages_in_range_mod3", "extract"),
    ("export_pages_in_range_mod3_as_pdf", "export_pdf"),
]

MOD3_SINGLE = [
    ("export_pages_in_range_mod3_png", "export_png", 0),
    ("export_pages_in_range_mod3_png", "export_png", 1),
    ("export_pages_in_range_mod3_png", "export_png", 2),
    ("export_pages_in_range_mod3_jpeg", "export_jpeg", 0),
]


def mod3_rust_helpers() -> str:
    body = INC1.read_text()
    start = body.index("fn parity_batch_indices")
    end = body.index("fn parity_batch_export_ico_doc")
    chunk = body[start:end]
    chunk = (
        "fn parity_mod3_match(page_index: u32, start_page: u32, end_page: u32, remainder: u32) -> bool {\n"
        "    if page_index < start_page || page_index > end_page {\n"
        "        return false;\n"
        "    }\n"
        "    (page_index - start_page) % 3 == remainder\n"
        "}\n\n" + chunk
    )
    chunk = chunk.replace("parity_batch_match", "parity_mod3_match")
    chunk = chunk.replace("parity_batch_", "parity_mod3_")
    chunk = re.sub(
        r"#\[allow\(clippy::manual_is_multiple_of\)\]\nfn parity_mod3_move_seg.*?^}\n",
        "",
        chunk,
        flags=re.MULTILINE | re.DOTALL,
    )
    chunk = re.sub(r"odd: bool, local: bool", "remainder: u32", chunk)
    chunk = chunk.replace("keep_odd: bool, local: bool", "remainder: u32")
    chunk = chunk.replace("keep_remainder: u32", "remainder: u32")
    chunk = chunk.replace(", odd, local)", ", remainder)")
    chunk = chunk.replace("parity_mod3_match(idx, start_page, end_page, keep_odd, local)", "parity_mod3_match(idx, start_page, end_page, remainder)")
    unused = [
        "parity_mod3_sort_rotation",
        "parity_mod3_sort_size",
        "parity_mod3_dup_before",
        "parity_mod3_dup_to_start",
        "parity_mod3_dup_to_end",
        "parity_mod3_expand",
        "parity_mod3_shrink",
        "parity_mod3_clear_crop",
        "parity_mod3_blank",
        "parity_mod3_bookmark",
        "parity_mod3_page_size",
        "parity_mod3_page_numbers",
        "parity_mod3_watermark",
        "parity_mod3_header",
        "parity_mod3_footer",
        "parity_mod3_border",
    ]
    for name in unused:
        chunk = re.sub(
            rf"(#\[allow\(clippy::too_many_arguments\)\]\n)?fn {name}\(.*?(?=\n(?:#\[allow|fn parity_mod3_))",
            "",
            chunk,
            flags=re.DOTALL,
        )
    return chunk


def local_impl_call(kind: str, odd: bool) -> str:
    o = "true" if odd else "false"
    path = "&PathBuf::from(&path)"
    rng = f"{path}, start_page, end_page"
    pl = f"{rng}, {o}, true"
    calls = {
        "rotate_cw": f"parity_batch_rotate({pl}, 90)",
        "rotate_ccw": f"parity_batch_rotate({pl}, -90)",
        "rotate_180": f"parity_batch_rotate({pl}, 180)",
        "reset_rot": f"parity_batch_reset_rotation({pl})",
        "delete": f"parity_batch_delete({pl})",
        "keep": f"parity_batch_keep({pl})",
        "dup_append": f"parity_batch_dup_append({pl})",
        "dup_before": f"parity_batch_dup_before({pl})",
        "dup_to_start": f"parity_batch_dup_to_start({pl})",
        "dup_to_end": f"parity_batch_dup_to_end({pl})",
        "flatten": f"parity_batch_flatten({pl})",
        "reverse": f"parity_batch_reverse({pl})",
        "move_odd_start": f"parity_batch_move_seg({rng}, true, true)",
        "move_even_start": f"parity_batch_move_seg({rng}, false, true)",
        "move_odd_end": f"parity_batch_move_seg({rng}, false, true)",
        "move_even_end": f"parity_batch_move_seg({rng}, true, true)",
        "sort_rot": f"parity_batch_sort_rotation({pl}, false)",
        "sort_size": f"parity_batch_sort_size({pl}, false)",
        "crop": f"parity_batch_crop({pl}, margin_top, margin_right, margin_bottom, margin_left)",
        "expand": f"parity_batch_expand({pl}, margin_top, margin_right, margin_bottom, margin_left)",
        "shrink": f"parity_batch_shrink({pl}, margin_top, margin_right, margin_bottom, margin_left)",
        "clear_crop": f"parity_batch_clear_crop({pl})",
        "blank_before": f"parity_batch_blank({pl}, false)",
        "blank_after": f"parity_batch_blank({pl}, true)",
        "bookmark": f"parity_batch_bookmark({pl}, prefix)",
        "page_size": f"parity_batch_page_size({pl}, &preset)",
        "extract": f"parity_batch_extract({path}, &PathBuf::from(&output_path), start_page, end_page, {o}, true)",
        "page_numbers": f"parity_batch_page_numbers({pl}, prefix)",
        "watermark": f"parity_batch_watermark({pl}, &text)",
        "header": f"parity_batch_header({pl}, &text)",
        "footer": f"parity_batch_footer({pl}, &text)",
        "border": f"parity_batch_border({pl}, inset)",
        "export_pdf": f"parity_batch_export_pdf({path}, &PathBuf::from(&output_dir), start_page, end_page, {o}, true)",
        "export_png": f'parity_batch_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {o}, true, "png", render_page_png)',
        "export_jpeg": f'parity_batch_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {o}, true, "jpeg", render_page_jpeg)',
        "export_webp": f'parity_batch_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {o}, true, "webp", render_page_webp)',
        "export_bmp": f'parity_batch_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {o}, true, "bmp", render_page_bmp)',
        "export_tiff": f'parity_batch_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {o}, true, "tiff", render_page_tiff)',
        "export_gif": f'parity_batch_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {o}, true, "gif", render_page_gif)',
        "export_ppm": f'parity_batch_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {o}, true, "ppm", render_page_ppm)',
        "export_tga": f'parity_batch_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {o}, true, "tga", render_page_tga)',
    }
    return calls[kind]


def mod3_impl_call(kind: str, remainder: int) -> str:
    r = str(remainder)
    path = "&PathBuf::from(&path)"
    rng = f"{path}, start_page, end_page"
    pl = f"{rng}, {r}"
    calls = {
        "rotate_cw": f"parity_mod3_rotate({pl}, 90)",
        "rotate_ccw": f"parity_mod3_rotate({pl}, -90)",
        "rotate_180": f"parity_mod3_rotate({pl}, 180)",
        "reset_rot": f"parity_mod3_reset_rotation({pl})",
        "delete": f"parity_mod3_delete({pl})",
        "keep": f"parity_mod3_keep({pl})",
        "dup_append": f"parity_mod3_dup_append({pl})",
        "flatten": f"parity_mod3_flatten({pl})",
        "reverse": f"parity_mod3_reverse({pl})",
        "crop": f"parity_mod3_crop({pl}, margin_top, margin_right, margin_bottom, margin_left)",
        "extract": f"parity_mod3_extract({path}, &PathBuf::from(&output_path), start_page, end_page, {r})",
        "export_pdf": f"parity_mod3_export_pdf({path}, &PathBuf::from(&output_dir), start_page, end_page, {r})",
        "export_png": f'parity_mod3_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {r}, "png", render_page_png)',
        "export_jpeg": f'parity_mod3_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {r}, "jpeg", render_page_jpeg)',
    }
    return calls[kind]


def command_sig(kind: str) -> tuple[str, str]:
    base = "path: String, start_page: u32, end_page: u32"
    ret = "Result<u32, String>"
    if kind == "extract":
        return f"{base}, output_path: String", "Result<String, String>"
    if kind.startswith("export"):
        return f"{base}, output_dir: String", "Result<Vec<String>, String>"
    if kind in ("crop", "expand", "shrink"):
        return f"{base}, margin_top: f64, margin_right: f64, margin_bottom: f64, margin_left: f64", ret
    if kind == "bookmark" or kind == "page_numbers":
        return f"{base}, prefix: Option<String>", ret
    if kind in ("watermark", "header", "footer"):
        return f"{base}, text: String", ret
    if kind == "page_size":
        return f"{base}, preset: String", ret
    if kind == "border":
        return f"{base}, inset: f64", ret
    if kind in ("move_odd_start", "move_even_start", "move_odd_end", "move_even_end"):
        return base, "Result<(), String>"
    return base, ret


def gen_local_command(name: str, kind: str, odd: bool) -> str:
    sig, ret = command_sig(kind)
    call = local_impl_call(kind, odd)
    label = "odd" if odd else "even"
    return f"""
/// Local {label} parity in range - {kind}
#[tauri::command]
fn {name}({sig}) -> {ret} {{
    {call}
}}
"""


def mod3_name(base: str, remainder: int) -> str:
    return base.replace("_pages", f"_mod3_{remainder}_pages")


def gen_mod3_command(name: str, kind: str, remainder: int) -> str:
    sig, ret = command_sig(kind)
    call = mod3_impl_call(kind, remainder)
    return f"""
/// Mod-3 remainder {remainder} in range - {kind}
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
    """Returns (name, kind, category) where category is local|mod3."""
    specs: list[tuple[str, str, str]] = []
    for base, kind in LOCAL_EXTENDED:
        if kind in ("move_odd_start", "move_even_start", "move_odd_end", "move_even_end"):
            specs.append((base, kind, "local"))
            continue
        for odd, label in ((True, "odd"), (False, "even")):
            name = base.replace("_pages", f"_{label}_pages")
            specs.append((name, kind, "local"))
    for base, kind in MOD3_TRIPLET:
        for rem in (0, 1, 2):
            specs.append((mod3_name(base, rem), kind, "mod3"))
    for base, kind, rem in MOD3_SINGLE:
        specs.append((mod3_name(base, rem), kind, "mod3"))
    return specs


def main() -> None:
    specs = build_specs()
    assert len(specs) == 100, len(specs)

    lines = [
        "// Auto-generated by scripts/gen-parity-batch2.py - do not edit.",
        mod3_rust_helpers(),
    ]
    tests = ["// Auto-generated parity batch2 tests"]
    handlers: list[str] = []

    for name, kind, cat in specs:
        if cat == "local":
            odd = "_odd_" in name
            lines.append(gen_local_command(name, kind, odd))
        else:
            rem = int(re.search(r"_mod3_(\d)_", name).group(1))  # type: ignore[union-attr]
            lines.append(gen_mod3_command(name, kind, rem))
        tests.append(gen_test(name, kind))
        handlers.append(f"            {name},")

    INC2.write_text("\n".join(lines))
    print(f"Wrote {INC2} ({len(specs)} commands)")

    import sys

    sys.path.insert(0, str(ROOT / "scripts"))
    from parity_patch import patch_sources

    patch_sources("BATCH2", handlers, tests)

    import importlib.util

    b1_path = ROOT / "scripts" / "gen-parity-batch.py"
    spec = importlib.util.spec_from_file_location("gen_parity_batch", b1_path)
    b1 = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(b1)
    base = [s[0] for s in b1.build_specs()]
    merged = base + [s[0] for s in specs]
    UI_JSON.write_text(json.dumps(merged, indent=2))
    print(f"Wrote {UI_JSON} ({len(merged)} total commands)")


if __name__ == "__main__":
    main()
