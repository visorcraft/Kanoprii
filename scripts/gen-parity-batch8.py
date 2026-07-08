#!/usr/bin/env python3
"""Generate sort-descending parity-in-range commands for all filter families."""

from __future__ import annotations

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
INC8 = ROOT / "src-tauri" / "src" / "parity_batch8_generated.inc.rs"
INC1 = ROOT / "src-tauri" / "src" / "parity_batch_generated.inc.rs"
MAIN = ROOT / "src-tauri" / "src" / "main.rs"
UI_JSON = ROOT / "src" / "parity_batch_commands.json"

MODULI = (3, 4, 5, 6)

SORT_KINDS = ("sort_rot", "sort_size")


def global_sort_name(kind: str, odd: bool) -> str:
    parity = "odd" if odd else "even"
    stem = "rotation" if kind == "sort_rot" else "size"
    return f"sort_{parity}_pages_in_range_by_{stem}_desc"


def local_sort_name(kind: str, odd: bool) -> str:
    parity = "odd" if odd else "even"
    stem = "rotation" if kind == "sort_rot" else "size"
    return f"sort_range_local_{parity}_pages_by_{stem}_desc"


def half_sort_name(kind: str, first_half: bool) -> str:
    label = "first_half" if first_half else "second_half"
    stem = "rotation" if kind == "sort_rot" else "size"
    return f"sort_{label}_pages_in_range_by_{stem}_desc"


def mod_sort_name(modulus: int, kind: str, remainder: int) -> str:
    stem = "rotation" if kind == "sort_rot" else "size"
    return f"sort_mod{modulus}_{remainder}_pages_in_range_mod{modulus}_by_{stem}_desc"


def batch1_helper_chunk() -> str:
    body = INC1.read_text()
    start = body.index("fn parity_batch_indices")
    end = body.index("fn parity_batch_export_ico_doc")
    return body[start:end]


def modn_sort_helpers(modulus: int) -> str:
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
    for short in ("sort_rotation", "sort_size"):
        fn = f"{prefix}{short}"
        m = re.search(
            rf"(#\[allow\(clippy::too_many_arguments\)\]\n)?fn {fn}\(.*?{end_lookahead}",
            chunk,
            flags=re.DOTALL,
        )
        if m:
            kept.append(m.group(0).rstrip())
    return "\n\n".join(kept) + ("\n\n" if kept else "")


def impl_call(filter_kind: str, sort_kind: str, **kw: object) -> str:
    path = "&PathBuf::from(&path)"
    rng = f"{path}, start_page, end_page"
    if filter_kind == "global":
        odd = "true" if kw["odd"] else "false"
        fn = "parity_batch_sort_rotation" if sort_kind == "sort_rot" else "parity_batch_sort_size"
        return f"{fn}({rng}, {odd}, false, true)"
    if filter_kind == "local":
        odd = "true" if kw["odd"] else "false"
        fn = "parity_batch_sort_rotation" if sort_kind == "sort_rot" else "parity_batch_sort_size"
        return f"{fn}({rng}, {odd}, true, true)"
    if filter_kind == "half":
        fh = "true" if kw["first_half"] else "false"
        fn = "parity_half_sort_rotation" if sort_kind == "sort_rot" else "parity_half_sort_size"
        return f"{fn}({rng}, {fh}, true)"
    modulus = int(kw["modulus"])
    rem = int(kw["remainder"])
    p = f"parity_mod{modulus}_"
    fn = f"{p}sort_rotation" if sort_kind == "sort_rot" else f"{p}sort_size"
    return f"{fn}({rng}, {rem}, true)"


def gen_command(name: str, filter_kind: str, sort_kind: str, label: str, **kw: object) -> str:
    call = impl_call(filter_kind, sort_kind, **kw)
    return f"""
/// Sort {label} by {'rotation' if sort_kind == 'sort_rot' else 'page size'} descending in range
#[tauri::command]
fn {name}(path: String, start_page: u32, end_page: u32) -> Result<u32, String> {{
    {call}
}}
"""


def gen_test(name: str) -> str:
    return f"""
    #[test]
    fn {name}_rejects_invalid_range() {{
        let path = save(&mut build_pdf(2), "{name}");
        let err = {name}(path.clone(), 5, 10).unwrap_err();
        assert!(err.contains("Invalid page range"));
        let _ = std::fs::remove_file(&path);
    }}
"""


def build_specs() -> list[tuple[str, str, str, dict]]:
    specs: list[tuple[str, str, str, dict]] = []
    for kind in SORT_KINDS:
        for odd in (True, False):
            name = global_sort_name(kind, odd)
            parity = "odd" if odd else "even"
            specs.append((name, "global", kind, {"odd": odd, "label": f"global {parity}"}))
        for odd in (True, False):
            name = local_sort_name(kind, odd)
            parity = "odd" if odd else "even"
            specs.append((name, "local", kind, {"odd": odd, "label": f"local {parity}"}))
        for first_half in (True, False):
            name = half_sort_name(kind, first_half)
            half = "first half" if first_half else "second half"
            specs.append((name, "half", kind, {"first_half": first_half, "label": half}))
    for modulus in MODULI:
        for rem in range(modulus):
            for kind in SORT_KINDS:
                name = mod_sort_name(modulus, kind, rem)
                specs.append((
                    name,
                    "mod",
                    kind,
                    {"modulus": modulus, "remainder": rem, "label": f"mod-{modulus} remainder {rem}"},
                ))
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
        "gen-parity-batch7.py",
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
    assert len(specs) == 48, len(specs)

    lines = ["// Auto-generated by scripts/gen-parity-batch8.py - do not edit."]
    for modulus in MODULI:
        lines.append(f"// --- mod-{modulus} sort helpers ---")
        lines.append(modn_sort_helpers(modulus))

    tests = ["// Auto-generated parity batch8 tests"]
    handlers: list[str] = []

    for name, filter_kind, sort_kind, kw in specs:
        label = str(kw.pop("label"))
        lines.append(gen_command(name, filter_kind, sort_kind, label, **kw))
        tests.append(gen_test(name))
        handlers.append(f"            {name},")

    INC8.write_text("\n".join(lines))
    print(f"Wrote {INC8} ({len(specs)} commands)")

    import sys

    sys.path.insert(0, str(ROOT / "scripts"))
    from parity_patch import patch_sources

    patch_sources("BATCH8", handlers, tests)

    merged = load_prior_command_names() + [s[0] for s in specs]
    UI_JSON.write_text(json.dumps(merged, indent=2))
    print(f"Wrote {UI_JSON} ({len(merged)} total commands)")


if __name__ == "__main__":
    main()
