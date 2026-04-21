#!/usr/bin/env python3
"""Inventory the current public `aspen-core` surface from `src/lib.rs`.

Initial scope for task 1.4: emit `surface-inventory.md` before no-std boundary
edits land. Later tasks can extend this checker with export-map and source-audit
rules without changing the command contract.
"""

from __future__ import annotations

import argparse
from dataclasses import dataclass
from pathlib import Path
import re
import sys

CFG_PREFIX = "#[cfg"
EMPTY_TEXT = ""
GUARD_NONE = "always"
LIB_RS_NAME = "lib.rs"
OUTPUT_FILE_NAME = "surface-inventory.md"
MODULE_PATTERN = re.compile(r"^pub\s+mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*;")
REEXPORT_PATTERN = re.compile(r"^pub\s+use\s+(.+?)\s*;")
ALIAS_SPLIT_PATTERN = re.compile(r"\s+as\s+")
GLOB_SUFFIX = "::*"


@dataclass(frozen=True)
class ModuleEntry:
    name: str
    guard: str


@dataclass(frozen=True)
class ReexportEntry:
    source: str
    exported_name: str
    guard: str


@dataclass(frozen=True)
class SurfaceInventory:
    crate_dir: Path
    lib_rs: Path
    modules: tuple[ModuleEntry, ...]
    reexports: tuple[ReexportEntry, ...]


class InventoryError(RuntimeError):
    pass


def normalized_line(raw_line: str) -> str:
    return raw_line.split("//", maxsplit=1)[0].strip()


def joined_guard(pending_attributes: list[str]) -> str:
    if not pending_attributes:
        return GUARD_NONE
    return " and ".join(pending_attributes)


def exported_name_for_source(source: str) -> str:
    if source.endswith(GLOB_SUFFIX):
        return "*"
    alias_parts = ALIAS_SPLIT_PATTERN.split(source, maxsplit=1)
    if len(alias_parts) == 2:
        return alias_parts[1].strip()
    tail = source.rsplit("::", maxsplit=1)
    return tail[-1].strip()


def parsed_inventory(crate_dir: Path) -> SurfaceInventory:
    lib_rs = crate_dir / LIB_RS_NAME
    if not lib_rs.is_file():
        raise InventoryError(f"missing {lib_rs}")

    modules: list[ModuleEntry] = []
    reexports: list[ReexportEntry] = []
    pending_attributes: list[str] = []

    for raw_line in lib_rs.read_text().splitlines():
        line = normalized_line(raw_line)
        if line == EMPTY_TEXT:
            continue
        if line.startswith(CFG_PREFIX):
            pending_attributes.append(line)
            continue

        module_match = MODULE_PATTERN.match(line)
        if module_match:
            modules.append(ModuleEntry(name=module_match.group(1), guard=joined_guard(pending_attributes)))
            pending_attributes.clear()
            continue

        reexport_match = REEXPORT_PATTERN.match(line)
        if reexport_match:
            source = reexport_match.group(1).strip()
            reexports.append(
                ReexportEntry(
                    source=source,
                    exported_name=exported_name_for_source(source),
                    guard=joined_guard(pending_attributes),
                )
            )
            pending_attributes.clear()
            continue

        if not line.startswith("#"):
            pending_attributes.clear()

    return SurfaceInventory(
        crate_dir=crate_dir,
        lib_rs=lib_rs,
        modules=tuple(modules),
        reexports=tuple(reexports),
    )


def module_table_rows(entries: tuple[ModuleEntry, ...]) -> list[str]:
    rows = ["| Module | Guard |", "| --- | --- |"]
    for entry in entries:
        rows.append(f"| `{entry.name}` | `{entry.guard}` |")
    return rows


def reexport_table_rows(entries: tuple[ReexportEntry, ...]) -> list[str]:
    rows = ["| Export | Source | Guard |", "| --- | --- | --- |"]
    for entry in entries:
        rows.append(f"| `{entry.exported_name}` | `{entry.source}` | `{entry.guard}` |")
    return rows


def markdown_for_inventory(inventory: SurfaceInventory) -> str:
    module_count = len(inventory.modules)
    reexport_count = len(inventory.reexports)
    sections = [
        "# Aspen Core Surface Inventory",
        EMPTY_TEXT,
        f"- Crate dir: `{inventory.crate_dir}`",
        f"- Source: `{inventory.lib_rs}`",
        f"- Public modules: `{module_count}`",
        f"- Root re-exports: `{reexport_count}`",
        EMPTY_TEXT,
        "## Public modules",
        *module_table_rows(inventory.modules),
        EMPTY_TEXT,
        "## Root re-exports",
        *reexport_table_rows(inventory.reexports),
        EMPTY_TEXT,
        "## Notes",
        "- Inventory source is declarative surface in `src/lib.rs` only.",
        "- Guards reflect contiguous `#[cfg(...)]` attributes immediately above each `pub mod` or `pub use` line.",
        "- This initial checker intentionally does not enforce policy yet; later tasks extend it with export-map and source-audit rules.",
        EMPTY_TEXT,
    ]
    return "\n".join(sections)


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--crate-dir", required=True, type=Path, help="Path to `crates/aspen-core/src`")
    parser.add_argument("--output-dir", required=True, type=Path, help="Directory for generated inventory artifacts")
    return parser.parse_args(argv)


def write_inventory(output_dir: Path, content: str) -> Path:
    output_dir.mkdir(parents=True, exist_ok=True)
    output_path = output_dir / OUTPUT_FILE_NAME
    output_path.write_text(content)
    return output_path


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    try:
        inventory = parsed_inventory(args.crate_dir)
    except InventoryError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    output_path = write_inventory(args.output_dir, markdown_for_inventory(inventory))
    print(output_path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
