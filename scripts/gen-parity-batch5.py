#!/usr/bin/env python3
"""Generate mod-4 extended in-range commands (mirror of mod-3 batches 36–39)."""

from __future__ import annotations

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
INC5 = ROOT / "src-tauri" / "src" / "parity_batch5_generated.inc.rs"
INC1 = ROOT / "src-tauri" / "src" / "parity_batch_generated.inc.rs"
MAIN = ROOT / "src-tauri" / "src" / "main.rs"
UI_JSON = ROOT / "src" / "parity_batch_commands.json"

MOD4_EXTENDED = [
    ("duplicate_pages_in_range_before_mod4", "dup_before"),
    ("duplicate_pages_in_range_to_start_mod4", "dup_to_start"),
    ("duplicate_pages_in_range_to_end_mod4", "dup_to_end"),
    ("expand_pages_in_range_mod4", "expand"),
    ("shrink_pages_in_range_mod4", "shrink"),
    ("clear_crop_pages_in_range_mod4", "clear_crop"),
    ("insert_blank_before_pages_in_range_mod4", "blank_before"),
    ("insert_blank_after_pages_in_range_mod4", "blank_after"),
    ("bookmark_pages_in_range_mod4", "bookmark"),
    ("set_page_size_pages_in_range_mod4", "page_size"),
    ("add_page_numbers_pages_in_range_mod4", "page_numbers"),
    ("add_text_watermark_pages_in_range_mod4", "watermark"),
    ("add_page_header_pages_in_range_mod4", "header"),
    ("add_page_footer_pages_in_range_mod4", "footer"),
    ("add_page_border_pages_in_range_mod4", "border"),
    ("export_pages_in_range_mod4_webp", "export_webp"),
    ("export_pages_in_range_mod4_bmp", "export_bmp"),
    ("export_pages_in_range_mod4_tiff", "export_tiff"),
    ("export_pages_in_range_mod4_gif", "export_gif"),
    ("export_pages_in_range_mod4_ppm", "export_ppm"),
    ("export_pages_in_range_mod4_tga", "export_tga"),
    ("export_pages_in_range_mod4_ico", "export_ico"),
]

MOD4_SUPPLEMENTAL = [
    "parity_mod4_dup_before",
    "parity_mod4_dup_to_start",
    "parity_mod4_dup_to_end",
    "parity_mod4_expand",
    "parity_mod4_shrink",
    "parity_mod4_clear_crop",
    "parity_mod4_blank",
    "parity_mod4_bookmark",
    "parity_mod4_page_size",
    "parity_mod4_page_numbers",
    "parity_mod4_watermark",
    "parity_mod4_header",
    "parity_mod4_footer",
    "parity_mod4_border",
]


def mod4_name(base: str, remainder: int) -> str:
    return base.replace("_pages", f"_mod4_{remainder}_pages")


def batch1_helper_chunk() -> str:
    body = INC1.read_text()
    start = body.index("fn parity_batch_indices")
    end = body.index("fn parity_batch_export_ico_doc")
    return body[start:end]


def mod4_supplemental_helpers() -> str:
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
    end_lookahead = r"(?=\n(?:#\[allow|fn parity_mod4_)|\Z)"
    kept: list[str] = []
    for name in MOD4_SUPPLEMENTAL:
        m = re.search(
            rf"(#\[allow\(clippy::too_many_arguments\)\]\n)?fn {name}\(.*?{end_lookahead}",
            chunk,
            flags=re.DOTALL,
        )
        if m:
            kept.append(m.group(0).rstrip())
    return "\n\n".join(kept) + ("\n\n" if kept else "")


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


def mod4_impl_call(kind: str, remainder: int) -> str:
    r = str(remainder)
    path = "&PathBuf::from(&path)"
    rng = f"{path}, start_page, end_page"
    pl = f"{rng}, {r}"
    calls = {
        "dup_before": f"parity_mod4_dup_before({pl})",
        "dup_to_start": f"parity_mod4_dup_to_start({pl})",
        "dup_to_end": f"parity_mod4_dup_to_end({pl})",
        "expand": f"parity_mod4_expand({pl}, margin_top, margin_right, margin_bottom, margin_left)",
        "shrink": f"parity_mod4_shrink({pl}, margin_top, margin_right, margin_bottom, margin_left)",
        "clear_crop": f"parity_mod4_clear_crop({pl})",
        "blank_before": f"parity_mod4_blank({pl}, false)",
        "blank_after": f"parity_mod4_blank({pl}, true)",
        "bookmark": f"parity_mod4_bookmark({pl}, prefix)",
        "page_size": f"parity_mod4_page_size({pl}, &preset)",
        "page_numbers": f"parity_mod4_page_numbers({pl}, prefix)",
        "watermark": f"parity_mod4_watermark({pl}, &text)",
        "header": f"parity_mod4_header({pl}, &text)",
        "footer": f"parity_mod4_footer({pl}, &text)",
        "border": f"parity_mod4_border({pl}, inset)",
        "export_webp": f'parity_mod4_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {r}, "webp", render_page_webp)',
        "export_bmp": f'parity_mod4_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {r}, "bmp", render_page_bmp)',
        "export_tiff": f'parity_mod4_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {r}, "tiff", render_page_tiff)',
        "export_gif": f'parity_mod4_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {r}, "gif", render_page_gif)',
        "export_ppm": f'parity_mod4_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {r}, "ppm", render_page_ppm)',
        "export_tga": f'parity_mod4_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {r}, "tga", render_page_tga)',
        "export_ico": f'parity_mod4_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {r}, "ico", render_page_ico)',
    }
    return calls[kind]


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
    if kind.startswith("export"):
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
    for base, kind in MOD4_EXTENDED:
        for rem in (0, 1, 2, 3):
            specs.append((mod4_name(base, rem), kind))
    return specs


def load_prior_command_names() -> list[str]:
    import importlib.util

    names: list[str] = []
    for script in (
        "gen-parity-batch.py",
        "gen-parity-batch2.py",
        "gen-parity-batch3.py",
        "gen-parity-batch4.py",
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
    assert len(specs) == 88, len(specs)

    lines = [
        "// Auto-generated by scripts/gen-parity-batch5.py - do not edit.",
        mod4_supplemental_helpers(),
    ]
    tests = ["// Auto-generated parity batch5 tests"]
    handlers: list[str] = []

    for name, kind in specs:
        rem = int(re.search(r"_mod4_(\d)_", name).group(1))  # type: ignore[union-attr]
        lines.append(gen_mod4_command(name, kind, rem))
        tests.append(gen_test(name, kind))
        handlers.append(f"            {name},")

    INC5.write_text("\n".join(lines))
    print(f"Wrote {INC5} ({len(specs)} commands)")

    import sys

    sys.path.insert(0, str(ROOT / "scripts"))
    from parity_patch import patch_sources

    patch_sources("BATCH5", handlers, tests)

    merged = load_prior_command_names() + [s[0] for s in specs]
    UI_JSON.write_text(json.dumps(merged, indent=2))
    print(f"Wrote {UI_JSON} ({len(merged)} total commands)")


if __name__ == "__main__":
    main()
