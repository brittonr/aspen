#!/usr/bin/env python3
"""Validate alloc-only `aspen-core` and sibling `aspen-core-shell` surfaces."""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass
from pathlib import Path

CFG_PREFIX = '#[cfg'
EMPTY_TEXT = ''
ALWAYS_GUARD = 'always'
LIB_RS_NAME = 'lib.rs'
CORE_SQL_GUARD = '#[cfg(feature = "sql")]'
SHELL_LAYER_GUARD = '#[cfg(feature = "layer")]'
TEST_GUARD = '#[cfg(test)]'
SURFACE_OUTPUT_FILE_NAME = 'surface-inventory.md'
EXPORT_MAP_FILE_NAME = 'export-map.md'
SOURCE_AUDIT_FILE_NAME = 'source-audit.txt'
MODULE_PATTERN = re.compile(r'^(pub(?:\([^)]*\))?)\s+mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*;')
REEXPORT_PATTERN = re.compile(r'^(pub(?:\([^)]*\))?)\s+use\s+(.+?)\s*;')
CORE_FORBIDDEN_GUARD_PATTERN = re.compile(r'feature = "(?:std|layer|global-discovery)"')
CORE_FORBIDDEN_EXPORT_PATTERN = re.compile(r'pub use (app_registry|context|layer|simulation|transport|utils)::')
CORE_ARC_IMPL_PATTERN = re.compile(r'impl<T: SqlQueryExecutor \+ \?Sized> SqlQueryExecutor for Arc<T>')
CORE_ALLOC_ARC_IMPORT_PATTERN = re.compile(r'use alloc::sync::Arc;')
SHELL_MODULE_REEXPORTS = {
    'circuit_breaker',
    'cluster',
    'crypto',
    'error',
    'hlc',
    'kv',
    'prelude',
    'protocol',
    'spec',
    'traits',
    'types',
    'vault',
    'verified',
}
CORE_EXPECTED_MODULES = {
    ('circuit_breaker', ALWAYS_GUARD),
    ('cluster', ALWAYS_GUARD),
    ('constants', ALWAYS_GUARD),
    ('crypto', ALWAYS_GUARD),
    ('error', ALWAYS_GUARD),
    ('hlc', ALWAYS_GUARD),
    ('kv', ALWAYS_GUARD),
    ('prelude', ALWAYS_GUARD),
    ('protocol', ALWAYS_GUARD),
    ('spec', ALWAYS_GUARD),
    ('sql', CORE_SQL_GUARD),
    ('storage', ALWAYS_GUARD),
    ('traits', ALWAYS_GUARD),
    ('types', ALWAYS_GUARD),
    ('vault', ALWAYS_GUARD),
    ('verified', ALWAYS_GUARD),
}
SHELL_EXPECTED_MODULES = {
    ('app_registry', ALWAYS_GUARD),
    ('constants', ALWAYS_GUARD),
    ('context', ALWAYS_GUARD),
    ('layer', SHELL_LAYER_GUARD),
    ('simulation', ALWAYS_GUARD),
    ('storage', ALWAYS_GUARD),
    ('transport', ALWAYS_GUARD),
    ('utils', ALWAYS_GUARD),
}
SHELL_EXPECTED_ROOT_EXPORTS = {
    'app_registry::AppRegistry',
    'context::EndpointProvider',
    'simulation::SimulationArtifact',
    'storage::SM_KV_TABLE',
    'transport::NetworkTransport',
    'utils::ensure_disk_space_available',
}
CORE_REMOVED_PATHS = (
    Path('app_registry.rs'),
    Path('context'),
    Path('layer'),
    Path('simulation.rs'),
    Path('transport.rs'),
    Path('utils.rs'),
)
CORE_STORAGE_FORBIDDEN_PATTERNS = (
    'SM_KV_TABLE',
    'TableDefinition::new',
)


@dataclass(frozen=True)
class SurfaceEntry:
    visibility: str
    source: str
    guard: str


class SurfaceError(RuntimeError):
    pass


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--crate-dir', required=True, type=Path, help='Path to crates/aspen-core/src')
    parser.add_argument('--output-dir', required=True, type=Path, help='Directory for generated artifacts')
    return parser.parse_args(argv)


def normalized_line(raw_line: str) -> str:
    return raw_line.split('//', maxsplit=1)[0].strip()


def joined_guard(pending_attributes: list[str]) -> str:
    if not pending_attributes:
        return ALWAYS_GUARD
    return ' and '.join(pending_attributes)


def parse_surface(file_path: Path) -> tuple[list[SurfaceEntry], list[SurfaceEntry]]:
    if not file_path.is_file():
        raise SurfaceError(f'missing {file_path}')
    modules: list[SurfaceEntry] = []
    reexports: list[SurfaceEntry] = []
    pending_attributes: list[str] = []
    for raw_line in file_path.read_text().splitlines():
        line = normalized_line(raw_line)
        if line == EMPTY_TEXT:
            continue
        if line.startswith(CFG_PREFIX):
            pending_attributes.append(line)
            continue
        module_match = MODULE_PATTERN.match(line)
        if module_match:
            visibility, name = module_match.groups()
            modules.append(SurfaceEntry(visibility=visibility, source=name, guard=joined_guard(pending_attributes)))
            pending_attributes.clear()
            continue
        reexport_match = REEXPORT_PATTERN.match(line)
        if reexport_match:
            visibility, source = reexport_match.groups()
            reexports.append(SurfaceEntry(visibility=visibility, source=source.strip(), guard=joined_guard(pending_attributes)))
            pending_attributes.clear()
            continue
        pending_attributes.clear()
    return modules, reexports


def public_modules(entries: list[SurfaceEntry]) -> set[tuple[str, str]]:
    return {(entry.source, entry.guard) for entry in entries if entry.visibility == 'pub'}


def public_reexports(entries: list[SurfaceEntry]) -> set[str]:
    return {entry.source for entry in entries if entry.visibility == 'pub'}


def render_table(title: str, rows: list[str]) -> list[str]:
    return [f'## {title}', *rows, EMPTY_TEXT]


def markdown_rows(entries: list[SurfaceEntry]) -> list[str]:
    rows = ['| Source | Guard |', '| --- | --- |']
    for entry in entries:
        if entry.visibility != 'pub':
            continue
        rows.append(f'| `{entry.source}` | `{entry.guard}` |')
    return rows


def validate_core(core_dir: Path) -> tuple[list[str], list[str], list[str]]:
    errors: list[str] = []
    notes: list[str] = []
    core_lib_path = core_dir / LIB_RS_NAME
    core_modules, core_reexports = parse_surface(core_lib_path)
    actual_modules = public_modules(core_modules)
    if actual_modules != CORE_EXPECTED_MODULES:
        missing_modules = sorted(CORE_EXPECTED_MODULES - actual_modules)
        unexpected_modules = sorted(actual_modules - CORE_EXPECTED_MODULES)
        for module_name, guard in missing_modules:
            errors.append(f'missing core module `{module_name}` guarded by `{guard}`')
        for module_name, guard in unexpected_modules:
            errors.append(f'unexpected core module `{module_name}` guarded by `{guard}`')
    test_support_matches = [entry for entry in core_modules if entry.source == 'test_support']
    if len(test_support_matches) != 1 or test_support_matches[0].visibility != 'pub(crate)' or test_support_matches[0].guard != TEST_GUARD:
        errors.append('core test_support must stay `pub(crate)` behind `#[cfg(test)]`')
    core_lib_text = core_lib_path.read_text()
    if CORE_FORBIDDEN_GUARD_PATTERN.search(core_lib_text) is not None:
        errors.append('core lib.rs must not mention std, layer, or global-discovery cfg gates')
    if CORE_FORBIDDEN_EXPORT_PATTERN.search(core_lib_text) is not None:
        errors.append('core lib.rs must not re-export shell-only paths')
    core_sql_path = core_dir / 'sql.rs'
    core_sql_text = core_sql_path.read_text()
    if CORE_ALLOC_ARC_IMPORT_PATTERN.search(core_sql_text) is None:
        errors.append('core sql.rs must import `alloc::sync::Arc` for blanket impls')
    if CORE_ARC_IMPL_PATTERN.search(core_sql_text) is None:
        errors.append('core sql.rs must keep the alloc-safe `Arc<T>` blanket impl')
    for removed_path in CORE_REMOVED_PATHS:
        absolute_path = core_dir / removed_path
        if absolute_path.exists():
            errors.append(f'core must not retain shell source path `{absolute_path}`')
    core_storage_text = (core_dir / 'storage.rs').read_text()
    for forbidden_pattern in CORE_STORAGE_FORBIDDEN_PATTERNS:
        if forbidden_pattern in core_storage_text:
            errors.append(f'core storage.rs must not mention `{forbidden_pattern}`')
    if not errors:
        notes.append('core crate contains only alloc-safe public modules and keeps shell families out of the source tree')
        notes.append('core storage surface keeps Redb table definitions out of alloc-only exports')
        notes.append('core sql blanket impl stays alloc-safe via `alloc::sync::Arc`')
    inventory_lines = [
        '# Aspen core surface inventory',
        EMPTY_TEXT,
        f'- Core crate dir: `{core_dir}`',
        EMPTY_TEXT,
        *render_table('Core public modules', markdown_rows(core_modules)),
        *render_table('Core public re-exports', markdown_rows(core_reexports)),
    ]
    return errors, notes, inventory_lines


def validate_shell(shell_dir: Path) -> tuple[list[str], list[str], list[str]]:
    errors: list[str] = []
    notes: list[str] = []
    shell_lib_path = shell_dir / LIB_RS_NAME
    shell_modules, shell_reexports = parse_surface(shell_lib_path)
    actual_modules = public_modules(shell_modules)
    if actual_modules != SHELL_EXPECTED_MODULES:
        missing_modules = sorted(SHELL_EXPECTED_MODULES - actual_modules)
        unexpected_modules = sorted(actual_modules - SHELL_EXPECTED_MODULES)
        for module_name, guard in missing_modules:
            errors.append(f'missing shell module `{module_name}` guarded by `{guard}`')
        for module_name, guard in unexpected_modules:
            errors.append(f'unexpected shell module `{module_name}` guarded by `{guard}`')
    reexport_sources = public_reexports(shell_reexports)
    for module_name in sorted(SHELL_MODULE_REEXPORTS):
        expected_reexport = f'aspen_core::{module_name}'
        if expected_reexport not in reexport_sources:
            errors.append(f'missing shell re-export `{expected_reexport}`')
    for export_source in sorted(SHELL_EXPECTED_ROOT_EXPORTS):
        if export_source not in reexport_sources:
            errors.append(f'missing shell root export `{export_source}`')
    if not errors:
        notes.append('shell crate owns runtime-only module families and re-exports alloc core modules for existing import paths')
        notes.append('shell storage module keeps `SM_KV_TABLE` on the shell side while preserving the public path')
        notes.append('layer remains a shell-only opt-in behind `feature = "layer"`')
    inventory_lines = [
        f'- Shell crate dir: `{shell_dir}`',
        EMPTY_TEXT,
        *render_table('Shell public modules', markdown_rows(shell_modules)),
        *render_table('Shell public re-exports', markdown_rows(shell_reexports)),
    ]
    return errors, notes, inventory_lines


def export_map(core_reexports: list[SurfaceEntry], shell_reexports: list[SurfaceEntry]) -> str:
    lines = [
        '# Aspen core export map',
        EMPTY_TEXT,
        '## Alloc-only root exports',
    ]
    for entry in core_reexports:
        if entry.visibility != 'pub':
            continue
        lines.append(f'- `{entry.source}`')
    lines.extend([
        EMPTY_TEXT,
        '## Shell root exports',
    ])
    for entry in shell_reexports:
        if entry.visibility != 'pub':
            continue
        lines.append(f'- `{entry.source}`')
    lines.append(EMPTY_TEXT)
    return '\n'.join(lines)


def source_audit(core_notes: list[str], shell_notes: list[str]) -> str:
    lines = [
        '# Aspen core source audit',
        EMPTY_TEXT,
        *[f'PASS {note}' for note in core_notes],
        *[f'PASS {note}' for note in shell_notes],
        EMPTY_TEXT,
    ]
    return '\n'.join(lines)


def write_outputs(output_dir: Path, surface_inventory: str, export_map_text: str, source_audit_text: str) -> tuple[Path, Path, Path]:
    output_dir.mkdir(parents=True, exist_ok=True)
    surface_path = output_dir / SURFACE_OUTPUT_FILE_NAME
    export_map_path = output_dir / EXPORT_MAP_FILE_NAME
    source_audit_path = output_dir / SOURCE_AUDIT_FILE_NAME
    surface_path.write_text(surface_inventory)
    export_map_path.write_text(export_map_text)
    source_audit_path.write_text(source_audit_text)
    return surface_path, export_map_path, source_audit_path


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    core_dir = args.crate_dir.resolve()
    shell_dir = core_dir.parent.parent / 'aspen-core-shell' / 'src'
    try:
        core_errors, core_notes, core_inventory = validate_core(core_dir)
        shell_errors, shell_notes, shell_inventory = validate_shell(shell_dir)
        _, core_reexports = parse_surface(core_dir / LIB_RS_NAME)
        _, shell_reexports = parse_surface(shell_dir / LIB_RS_NAME)
    except SurfaceError as error:
        print(f'error: {error}', file=sys.stderr)
        return 1
    validation_errors = [*core_errors, *shell_errors]
    if validation_errors:
        for error in validation_errors:
            print(f'error: {error}', file=sys.stderr)
        return 1
    surface_inventory = '\n'.join([*core_inventory, *shell_inventory]) + '\n'
    export_map_text = export_map(core_reexports, shell_reexports)
    source_audit_text = source_audit(core_notes, shell_notes)
    surface_path, export_map_path, source_audit_path = write_outputs(args.output_dir, surface_inventory, export_map_text, source_audit_text)
    print(surface_path)
    print(export_map_path)
    print(source_audit_path)
    return 0


if __name__ == '__main__':
    raise SystemExit(main(sys.argv[1:]))
