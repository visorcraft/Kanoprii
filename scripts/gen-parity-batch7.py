#!/usr/bin/env python3
"""Generate third-range in-range parity commands (first/second/third third of range)."""

from __future__ import annotations

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
INC7 = ROOT / "src-tauri" / "src" / "parity_batch7_generated.inc.rs"
INC1 = ROOT / "src-tauri" / "src" / "parity_batch_generated.inc.rs"
MAIN = ROOT / "src-tauri" / "src" / "main.rs"
UI_JSON = ROOT / "src" / "parity_batch_commands.json"

THIRD_LABELS = ("first_third", "second_third", "third_third")
THIRD_TITLES = ("first third", "second third", "third third")

THIRD_RANGE = [
    ("rotate_pages_in_range", "rotate_cw"),
    ("rotate_pages_in_range_ccw", "rotate_ccw"),
    ("rotate_180_pages_in_range", "rotate_180"),
    ("reset_rotation_pages_in_range", "reset_rot"),
    ("delete_pages_in_range", "delete"),
    ("keep_pages_in_range", "keep"),
    ("duplicate_pages_in_range", "dup_append"),
    ("flatten_pages_in_range", "flatten"),
    ("reverse_pages_in_range", "reverse"),
    ("crop_pages_in_range", "crop"),
    ("extract_pages_in_range", "extract"),
    ("export_pages_in_range_as_pdf", "export_pdf"),
    ("export_pages_in_range_png", "export_png"),
    ("export_pages_in_range_jpeg", "export_jpeg"),
    ("add_page_numbers_pages_in_range", "page_numbers"),
    ("add_text_watermark_pages_in_range", "watermark"),
]

THIRD_EXTENDED = [
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

THIRD_CORE_HELPERS = [
    "parity_third_indices",
    "parity_third_rotate",
    "parity_third_reset_rotation",
    "parity_third_delete",
    "parity_third_keep",
    "parity_third_dup_append",
    "parity_third_flatten",
    "parity_third_reverse",
    "parity_third_crop",
    "parity_third_extract",
    "parity_third_export_pdf",
    "parity_third_export_rendered",
    "parity_third_page_numbers",
    "parity_third_watermark",
]

THIRD_SUPPLEMENTAL = [
    "parity_third_dup_before",
    "parity_third_dup_to_start",
    "parity_third_dup_to_end",
    "parity_third_expand",
    "parity_third_shrink",
    "parity_third_clear_crop",
    "parity_third_blank",
    "parity_third_bookmark",
    "parity_third_page_size",
    "parity_third_header",
    "parity_third_footer",
    "parity_third_border",
    "parity_third_sort_rotation",
    "parity_third_sort_size",
]

THIRD_MATCH_PREAMBLE = """fn parity_third_match(page_index: u32, start_page: u32, end_page: u32, third: u32) -> bool {
    if page_index < start_page || page_index > end_page {
        return false;
    }
    let span = end_page - start_page;
    let t1 = start_page + span / 3;
    let t2 = start_page + (2 * span) / 3;
    match third {
        0 => page_index <= t1,
        1 => page_index > t1 && page_index <= t2,
        2 => page_index > t2,
        _ => false,
    }
}
"""


def third_name(base: str, third: int) -> str:
    return base.replace("_pages", f"_{THIRD_LABELS[third]}_pages", 1)


def batch1_helper_chunk() -> str:
    body = INC1.read_text()
    start = body.index("fn parity_batch_indices")
    end = body.index("fn parity_batch_export_ico_doc")
    return body[start:end]


def transform_third_chunk(chunk: str) -> str:
    chunk = THIRD_MATCH_PREAMBLE + "\n\n" + chunk
    chunk = chunk.replace("parity_batch_match", "parity_third_match")
    chunk = chunk.replace("parity_batch_", "parity_third_")
    chunk = re.sub(r"odd: bool, local: bool", "third: u32", chunk)
    chunk = chunk.replace("keep_odd: bool, local: bool", "keep_third: u32")
    chunk = chunk.replace(", odd, local)", ", third)")
    chunk = chunk.replace(
        "parity_third_match(idx, start_page, end_page, keep_odd, local)",
        "parity_third_match(idx, start_page, end_page, keep_third)",
    )
    return chunk


def extract_third_fns(chunk: str, names: list[str]) -> str:
    end_lookahead = r"(?=\n(?:#\[allow|fn parity_third_)|\Z)"
    kept: list[str] = []
    for name in names:
        m = re.search(
            rf"(#\[allow\(clippy::too_many_arguments\)\]\n)?fn {name}\(.*?{end_lookahead}",
            chunk,
            flags=re.DOTALL,
        )
        if m:
            kept.append(m.group(0).rstrip())
    return "\n\n".join(kept) + ("\n\n" if kept else "")


def third_core_helpers() -> str:
    chunk = transform_third_chunk(batch1_helper_chunk())
    for name in [
        "parity_third_move_seg",
        "parity_third_sort_rotation",
        "parity_third_sort_size",
        "parity_third_dup_before",
        "parity_third_dup_to_start",
        "parity_third_dup_to_end",
        "parity_third_expand",
        "parity_third_shrink",
        "parity_third_clear_crop",
        "parity_third_blank",
        "parity_third_bookmark",
        "parity_third_page_size",
        "parity_third_header",
        "parity_third_footer",
        "parity_third_border",
    ]:
        chunk = re.sub(
            rf"(#\[allow\(clippy::too_many_arguments\)\]\n)?fn {name}\(.*?(?=\n(?:#\[allow|fn parity_third_))",
            "",
            chunk,
            flags=re.DOTALL,
        )
    return THIRD_MATCH_PREAMBLE + "\n\n" + extract_third_fns(chunk, THIRD_CORE_HELPERS).strip()


def third_supplemental_helpers() -> str:
    chunk = transform_third_chunk(batch1_helper_chunk())
    return extract_third_fns(chunk, THIRD_SUPPLEMENTAL)


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


def third_impl_call(kind: str, third: int) -> str:
    t = str(third)
    path = "&PathBuf::from(&path)"
    rng = f"{path}, start_page, end_page"
    pl = f"{rng}, {t}"
    calls = {
        "rotate_cw": f"parity_third_rotate({pl}, 90)",
        "rotate_ccw": f"parity_third_rotate({pl}, -90)",
        "rotate_180": f"parity_third_rotate({pl}, 180)",
        "reset_rot": f"parity_third_reset_rotation({pl})",
        "delete": f"parity_third_delete({pl})",
        "keep": f"parity_third_keep({pl})",
        "dup_append": f"parity_third_dup_append({pl})",
        "flatten": f"parity_third_flatten({pl})",
        "reverse": f"parity_third_reverse({pl})",
        "crop": f"parity_third_crop({pl}, margin_top, margin_right, margin_bottom, margin_left)",
        "extract": f"parity_third_extract({path}, &PathBuf::from(&output_path), start_page, end_page, {t})",
        "export_pdf": f"parity_third_export_pdf({path}, &PathBuf::from(&output_dir), start_page, end_page, {t})",
        "export_png": f'parity_third_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {t}, "png", render_page_png)',
        "export_jpeg": f'parity_third_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {t}, "jpeg", render_page_jpeg)',
        "page_numbers": f"parity_third_page_numbers({pl}, prefix)",
        "watermark": f"parity_third_watermark({pl}, &text)",
        "dup_before": f"parity_third_dup_before({pl})",
        "dup_to_start": f"parity_third_dup_to_start({pl})",
        "dup_to_end": f"parity_third_dup_to_end({pl})",
        "expand": f"parity_third_expand({pl}, margin_top, margin_right, margin_bottom, margin_left)",
        "shrink": f"parity_third_shrink({pl}, margin_top, margin_right, margin_bottom, margin_left)",
        "clear_crop": f"parity_third_clear_crop({pl})",
        "blank_before": f"parity_third_blank({pl}, false)",
        "blank_after": f"parity_third_blank({pl}, true)",
        "bookmark": f"parity_third_bookmark({pl}, prefix)",
        "page_size": f"parity_third_page_size({pl}, &preset)",
        "header": f"parity_third_header({pl}, &text)",
        "footer": f"parity_third_footer({pl}, &text)",
        "border": f"parity_third_border({pl}, inset)",
        "export_webp": f'parity_third_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {t}, "webp", render_page_webp)',
        "export_bmp": f'parity_third_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {t}, "bmp", render_page_bmp)',
        "export_tiff": f'parity_third_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {t}, "tiff", render_page_tiff)',
        "export_gif": f'parity_third_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {t}, "gif", render_page_gif)',
        "export_ppm": f'parity_third_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {t}, "ppm", render_page_ppm)',
        "export_tga": f'parity_third_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {t}, "tga", render_page_tga)',
        "export_ico": f'parity_third_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {t}, "ico", render_page_ico)',
        "sort_rot": f"parity_third_sort_rotation({pl}, false)",
        "sort_size": f"parity_third_sort_size({pl}, false)",
    }
    return calls[kind]


def gen_third_command(name: str, kind: str, third: int) -> str:
    sig, ret = command_sig(kind)
    call = third_impl_call(kind, third)
    title = THIRD_TITLES[third]
    return f"""
/// {title.capitalize()} of page range - {kind}
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


def build_specs() -> list[tuple[str, str]]:
    specs: list[tuple[str, str]] = []
    for base, kind in THIRD_RANGE + THIRD_EXTENDED:
        for third in range(3):
            specs.append((third_name(base, third), kind))
    return specs


def load_prior_command_names() -> list[str]:
    import importlib.util

    names: list[str] = []
    for script in (
        "gen-parity-batch.py",
        "gen-parity-batch2.py",
        "gen-parity-batch3.py",
        "gen-parity-batch4.py",
        "gen-parity-batch5.py",
        "gen-parity-batch6.py",
    ):
        path = ROOT / "scripts" / script
        spec = importlib.util.spec_from_file_location(script, path)
        mod = importlib.util.module_from_spec(spec)
        assert spec.loader is not None
        spec.loader.exec_module(mod)
        names.extend(s[0] for s in mod.build_specs())
    return names


def main() -> None:
    specs = build_specs()
    assert len(specs) == 114, len(specs)

    lines = [
        "// Auto-generated by scripts/gen-parity-batch7.py - do not edit.",
        third_core_helpers(),
        third_supplemental_helpers(),
    ]
    tests = ["// Auto-generated parity batch7 tests"]
    handlers: list[str] = []

    for name, kind in specs:
        third = THIRD_LABELS.index(next(l for l in THIRD_LABELS if f"_{l}_" in name))
        lines.append(gen_third_command(name, kind, third))
        tests.append(gen_test(name, kind))
        handlers.append(f"            {name},")

    INC7.write_text("\n".join(lines))
    print(f"Wrote {INC7} ({len(specs)} commands)")

    import sys

    sys.path.insert(0, str(ROOT / "scripts"))
    from parity_patch import patch_sources

    patch_sources("BATCH7", handlers, tests)

    merged = load_prior_command_names() + [s[0] for s in specs]
    UI_JSON.write_text(json.dumps(merged, indent=2))
    print(f"Wrote {UI_JSON} ({len(merged)} total commands)")


if __name__ == "__main__":
    main()
