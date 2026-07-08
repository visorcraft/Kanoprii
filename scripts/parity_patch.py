"""Shared patch targets for the gen-parity-*.py generators.

After the main.rs split, generated parity code lands in three files:
  - include! lines:   src-tauri/src/main.rs            (// PARITY_<TAG>_INCLUDE)
  - handler entries:  src-tauri/src/commands/invoke_handler.inc.rs
                                                       (// PARITY_<TAG>_HANDLERS_START/END)
  - tests:            src-tauri/src/main_tests.rs      (// PARITY_<TAG>_TESTS_START/END)

All markers must already exist; a missing marker aborts instead of silently
regenerating nothing. Run `cargo fmt` after regenerating.
"""

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MAIN = ROOT / "src-tauri" / "src" / "main.rs"
HANDLERS_INC = ROOT / "src-tauri" / "src" / "commands" / "invoke_handler.inc.rs"
TESTS_FILE = ROOT / "src-tauri" / "src" / "main_tests.rs"


def _replace_between(text: str, start: str, end: str, block: str, path: Path) -> str:
    if start not in text or end not in text:
        raise SystemExit(f"error: markers {start} / {end} not found in {path}")
    return re.sub(
        rf"[ \t]*{re.escape(start)}.*?{re.escape(end)}",
        lambda _m: block,
        text,
        count=1,
        flags=re.DOTALL,
    )


def ensure_include(tag: str) -> None:
    marker = f"// PARITY_{tag}_INCLUDE"
    if marker not in MAIN.read_text():
        raise SystemExit(
            f"error: {marker} not found in {MAIN}; "
            f"add the include! for the generated file manually"
        )


def patch_handlers(tag: str, handlers: list[str]) -> None:
    start = f"// PARITY_{tag}_HANDLERS_START"
    end = f"// PARITY_{tag}_HANDLERS_END"
    block = f"            {start}\n" + "\n".join(handlers) + f"\n            {end}"
    text = _replace_between(HANDLERS_INC.read_text(), start, end, block, HANDLERS_INC)
    HANDLERS_INC.write_text(text)
    print(f"Patched {HANDLERS_INC} ({tag} handlers)")


def patch_tests(tag: str, tests: list[str]) -> None:
    start = f"// PARITY_{tag}_TESTS_START"
    end = f"// PARITY_{tag}_TESTS_END"
    body = "\n".join(tests)
    # gen_test blocks are written for the old nested `mod tests`; main_tests.rs
    # items live at column 0, so strip one level of indentation.
    body = re.sub(r"(?m)^    ", "", body)
    block = f"{start}\n{body}\n{end}"
    text = _replace_between(TESTS_FILE.read_text(), start, end, block, TESTS_FILE)
    TESTS_FILE.write_text(text)
    print(f"Patched {TESTS_FILE} ({tag} tests)")


def patch_sources(tag: str, handlers: list[str], tests: list[str]) -> None:
    ensure_include(tag)
    patch_handlers(tag, handlers)
    patch_tests(tag, tests)
