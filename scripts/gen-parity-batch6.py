#!/usr/bin/env python3
"""Generate mod-5 and mod-6 core + extended in-range parity commands."""

from __future__ import annotations

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
INC6 = ROOT / "src-tauri" / "src" / "parity_batch6_generated.inc.rs"
INC1 = ROOT / "src-tauri" / "src" / "parity_batch_generated.inc.rs"
MAIN = ROOT / "src-tauri" / "src" / "main.rs"
UI_JSON = ROOT / "src" / "parity_batch_commands.json"

MODULI = (5, 6)

MODN_CORE = [
    ("rotate_pages_in_range_mod{n}", "rotate_cw"),
    ("rotate_pages_in_range_mod{n}_ccw", "rotate_ccw"),
    ("rotate_180_pages_in_range_mod{n}", "rotate_180"),
    ("reset_rotation_pages_in_range_mod{n}", "reset_rot"),
    ("delete_pages_in_range_mod{n}", "delete"),
    ("keep_pages_in_range_mod{n}", "keep"),
    ("duplicate_pages_in_range_mod{n}", "dup_append"),
    ("flatten_pages_in_range_mod{n}", "flatten"),
    ("reverse_pages_in_range_mod{n}", "reverse"),
    ("crop_pages_in_range_mod{n}", "crop"),
    ("extract_pages_in_range_mod{n}", "extract"),
    ("export_pages_in_range_mod{n}_as_pdf", "export_pdf"),
    ("export_pages_in_range_mod{n}_png", "export_png"),
    ("export_pages_in_range_mod{n}_jpeg", "export_jpeg"),
]

MODN_EXTENDED = [
    ("duplicate_pages_in_range_before_mod{n}", "dup_before"),
    ("duplicate_pages_in_range_to_start_mod{n}", "dup_to_start"),
    ("duplicate_pages_in_range_to_end_mod{n}", "dup_to_end"),
    ("expand_pages_in_range_mod{n}", "expand"),
    ("shrink_pages_in_range_mod{n}", "shrink"),
    ("clear_crop_pages_in_range_mod{n}", "clear_crop"),
    ("insert_blank_before_pages_in_range_mod{n}", "blank_before"),
    ("insert_blank_after_pages_in_range_mod{n}", "blank_after"),
    ("bookmark_pages_in_range_mod{n}", "bookmark"),
    ("set_page_size_pages_in_range_mod{n}", "page_size"),
    ("add_page_numbers_pages_in_range_mod{n}", "page_numbers"),
    ("add_text_watermark_pages_in_range_mod{n}", "watermark"),
    ("add_page_header_pages_in_range_mod{n}", "header"),
    ("add_page_footer_pages_in_range_mod{n}", "footer"),
    ("add_page_border_pages_in_range_mod{n}", "border"),
    ("export_pages_in_range_mod{n}_webp", "export_webp"),
    ("export_pages_in_range_mod{n}_bmp", "export_bmp"),
    ("export_pages_in_range_mod{n}_tiff", "export_tiff"),
    ("export_pages_in_range_mod{n}_gif", "export_gif"),
    ("export_pages_in_range_mod{n}_ppm", "export_ppm"),
    ("export_pages_in_range_mod{n}_tga", "export_tga"),
    ("export_pages_in_range_mod{n}_ico", "export_ico"),
]

MODN_CORE_HELPERS = [
    "indices",
    "rotate",
    "reset_rotation",
    "delete",
    "keep",
    "dup_append",
    "flatten",
    "reverse",
    "crop",
    "extract",
    "export_pdf",
    "export_rendered",
]

MODN_SUPPLEMENTAL = [
    "dup_before",
    "dup_to_start",
    "dup_to_end",
    "expand",
    "shrink",
    "clear_crop",
    "blank",
    "bookmark",
    "page_size",
    "page_numbers",
    "watermark",
    "header",
    "footer",
    "border",
]


def modn_name(base: str, modulus: int, remainder: int) -> str:
    return base.replace("_pages", f"_mod{modulus}_{remainder}_pages")


def batch1_helper_chunk() -> str:
    body = INC1.read_text()
    start = body.index("fn parity_batch_indices")
    end = body.index("fn parity_batch_export_ico_doc")
    return body[start:end]


def modn_core_helpers(modulus: int) -> str:
    prefix = f"parity_mod{modulus}_"
    chunk = batch1_helper_chunk()
    chunk = (
        f"fn {prefix}match(page_index: u32, start_page: u32, end_page: u32, remainder: u32) -> bool {{\n"
        "    if page_index < start_page || page_index > end_page {\n"
        "        return false;\n"
        "    }\n"
        f"    (page_index - start_page) % {modulus} == remainder\n"
        "}}\n\n" + chunk
    )
    chunk = chunk.replace("parity_batch_match", f"{prefix}match")
    chunk = chunk.replace("parity_batch_", prefix)
    chunk = re.sub(r"odd: bool, local: bool", "remainder: u32", chunk)
    chunk = chunk.replace("keep_remainder: u32", "remainder: u32")
    chunk = chunk.replace(", odd, local)", ", remainder)")
    chunk = chunk.replace(
        f"{prefix}match(idx, start_page, end_page, keep_odd, local)",
        f"{prefix}match(idx, start_page, end_page, remainder)",
    )
    chunk = re.sub(
        rf"#\[allow\(clippy::manual_is_multiple_of\)\]\nfn {prefix}move_seg.*?^}}\n",
        "",
        chunk,
        flags=re.MULTILINE | re.DOTALL,
    )
    keep = {f"{prefix}{name}" for name in MODN_CORE_HELPERS}
    keep.add(f"{prefix}match")
    unused = [name for name in re.findall(rf"fn ({prefix}\w+)", chunk) if name not in keep]
    for name in unused:
        chunk = re.sub(
            rf"(#\[allow\(clippy::too_many_arguments\)\]\n)?fn {name}\(.*?(?=\n(?:#\[allow|fn {prefix}))",
            "",
            chunk,
            flags=re.DOTALL,
        )
    end_lookahead = rf"(?=\n(?:#\[allow|fn {prefix})|\Z)"
    preamble = (
        f"fn {prefix}match(page_index: u32, start_page: u32, end_page: u32, remainder: u32) -> bool {{\n"
        "    if page_index < start_page || page_index > end_page {\n"
        "        return false;\n"
        "    }\n"
        f"    (page_index - start_page) % {modulus} == remainder\n"
        "}"
    )
    kept: list[str] = []
    for name in MODN_CORE_HELPERS:
        fn = f"{prefix}{name}"
        m = re.search(
            rf"(#\[allow\(clippy::too_many_arguments\)\]\n)?fn {fn}\(.*?{end_lookahead}",
            chunk,
            flags=re.DOTALL,
        )
        if m:
            kept.append(m.group(0).rstrip())
    body = "\n\n".join(kept)
    return preamble + ("\n\n" + body if body else "\n")


def modn_supplemental_helpers(modulus: int) -> str:
    prefix = f"parity_mod{modulus}_"
    chunk = batch1_helper_chunk()
    chunk = (
        f"fn {prefix}match(page_index: u32, start_page: u32, end_page: u32, remainder: u32) -> bool {{\n"
        "    if page_index < start_page || page_index > end_page {\n"
        "        return false;\n"
        "    }\n"
        f"    (page_index - start_page) % {modulus} == remainder\n"
        "}}\n\n" + chunk
    )
    chunk = chunk.replace("parity_batch_match", f"{prefix}match")
    chunk = chunk.replace("parity_batch_", prefix)
    chunk = re.sub(r"odd: bool, local: bool", "remainder: u32", chunk)
    chunk = chunk.replace("keep_remainder: u32", "remainder: u32")
    chunk = chunk.replace(", odd, local)", ", remainder)")
    chunk = chunk.replace(
        f"{prefix}match(idx, start_page, end_page, keep_odd, local)",
        f"{prefix}match(idx, start_page, end_page, remainder)",
    )
    end_lookahead = rf"(?=\n(?:#\[allow|fn {prefix})|\Z)"
    kept: list[str] = []
    for short in MODN_SUPPLEMENTAL:
        fn = f"{prefix}{short}"
        m = re.search(
            rf"(#\[allow\(clippy::too_many_arguments\)\]\n)?fn {fn}\(.*?{end_lookahead}",
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


def modn_impl_call(modulus: int, kind: str, remainder: int) -> str:
    r = str(remainder)
    p = f"parity_mod{modulus}_"
    path = "&PathBuf::from(&path)"
    rng = f"{path}, start_page, end_page"
    pl = f"{rng}, {r}"
    calls = {
        "rotate_cw": f"{p}rotate({pl}, 90)",
        "rotate_ccw": f"{p}rotate({pl}, -90)",
        "rotate_180": f"{p}rotate({pl}, 180)",
        "reset_rot": f"{p}reset_rotation({pl})",
        "delete": f"{p}delete({pl})",
        "keep": f"{p}keep({pl})",
        "dup_append": f"{p}dup_append({pl})",
        "flatten": f"{p}flatten({pl})",
        "reverse": f"{p}reverse({pl})",
        "crop": f"{p}crop({pl}, margin_top, margin_right, margin_bottom, margin_left)",
        "extract": f"{p}extract({path}, &PathBuf::from(&output_path), start_page, end_page, {r})",
        "export_pdf": f"{p}export_pdf({path}, &PathBuf::from(&output_dir), start_page, end_page, {r})",
        "export_png": f'{p}export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {r}, "png", render_page_png)',
        "export_jpeg": f'{p}export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {r}, "jpeg", render_page_jpeg)',
        "dup_before": f"{p}dup_before({pl})",
        "dup_to_start": f"{p}dup_to_start({pl})",
        "dup_to_end": f"{p}dup_to_end({pl})",
        "expand": f"{p}expand({pl}, margin_top, margin_right, margin_bottom, margin_left)",
        "shrink": f"{p}shrink({pl}, margin_top, margin_right, margin_bottom, margin_left)",
        "clear_crop": f"{p}clear_crop({pl})",
        "blank_before": f"{p}blank({pl}, false)",
        "blank_after": f"{p}blank({pl}, true)",
        "bookmark": f"{p}bookmark({pl}, prefix)",
        "page_size": f"{p}page_size({pl}, &preset)",
        "page_numbers": f"{p}page_numbers({pl}, prefix)",
        "watermark": f"{p}watermark({pl}, &text)",
        "header": f"{p}header({pl}, &text)",
        "footer": f"{p}footer({pl}, &text)",
        "border": f"{p}border({pl}, inset)",
        "export_webp": f'{p}export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {r}, "webp", render_page_webp)',
        "export_bmp": f'{p}export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {r}, "bmp", render_page_bmp)',
        "export_tiff": f'{p}export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {r}, "tiff", render_page_tiff)',
        "export_gif": f'{p}export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {r}, "gif", render_page_gif)',
        "export_ppm": f'{p}export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {r}, "ppm", render_page_ppm)',
        "export_tga": f'{p}export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {r}, "tga", render_page_tga)',
        "export_ico": f'{p}export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {r}, "ico", render_page_ico)',
    }
    return calls[kind]


def gen_modn_command(name: str, modulus: int, kind: str, remainder: int) -> str:
    sig, ret = command_sig(kind)
    call = modn_impl_call(modulus, kind, remainder)
    return f"""
/// Mod-{modulus} remainder {remainder} in range - {kind}
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


def build_specs() -> list[tuple[str, int, str]]:
    specs: list[tuple[str, int, str]] = []
    for modulus in MODULI:
        for base_t, kind in MODN_CORE:
            base = base_t.format(n=modulus)
            for rem in range(modulus):
                specs.append((modn_name(base, modulus, rem), modulus, kind))
        for base_t, kind in MODN_EXTENDED:
            base = base_t.format(n=modulus)
            for rem in range(modulus):
                specs.append((modn_name(base, modulus, rem), modulus, kind))
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
    expected = sum(n * (14 + 22) for n in MODULI)
    assert len(specs) == expected, (len(specs), expected)

    lines = ["// Auto-generated by scripts/gen-parity-batch6.py - do not edit."]
    for modulus in MODULI:
        lines.append(f"// --- mod-{modulus} core helpers ---")
        lines.append(modn_core_helpers(modulus))
        lines.append(f"// --- mod-{modulus} extended helpers ---")
        lines.append(modn_supplemental_helpers(modulus))

    tests = ["// Auto-generated parity batch6 tests"]
    handlers: list[str] = []

    for name, modulus, kind in specs:
        rem = int(re.search(rf"_mod{modulus}_(\d)_", name).group(1))  # type: ignore[union-attr]
        lines.append(gen_modn_command(name, modulus, kind, rem))
        tests.append(gen_test(name, kind))
        handlers.append(f"            {name},")

    INC6.write_text("\n".join(lines))
    print(f"Wrote {INC6} ({len(specs)} commands)")

    import sys

    sys.path.insert(0, str(ROOT / "scripts"))
    from parity_patch import patch_sources

    patch_sources("BATCH6", handlers, tests)

    merged = load_prior_command_names() + [s[0] for s in specs]
    UI_JSON.write_text(json.dumps(merged, indent=2))
    print(f"Wrote {UI_JSON} ({len(merged)} total commands)")


if __name__ == "__main__":
    main()
