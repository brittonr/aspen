#!/usr/bin/env python3
"""Check alloc-only and shell feature claims for Aspen core crates."""

from __future__ import annotations

import argparse
import json
import sys
import tomllib
from pathlib import Path
from typing import Any

EMPTY_LIST: list[str] = []
CORE_FEATURE_MARKER = 'aspen-core feature "'
SHELL_DEFAULT_FEATURE_MARKER = 'aspen-core-shell feature "default"'
SHELL_LAYER_FEATURE_MARKER = 'aspen-core-shell feature "layer"'
NO_STD_MARKER = '#![no_std]'
ALLOC_MARKER = 'extern crate alloc;'
ASPEN_CORE_MARKER = 'aspen_core::'
CORE_DEFAULT_FEATURES = frozenset({'default', 'sql'})
CORE_SQL_FEATURES = frozenset({'aspen-constants/sql'})
SHELL_DEFAULT_FEATURES = frozenset({'default', 'sql', 'layer', 'global-discovery'})
SHELL_SQL_FEATURES = frozenset({'aspen-core/sql'})
SHELL_LAYER_FEATURES = frozenset({'dep:aspen-layer'})
SHELL_GLOBAL_DISCOVERY_FEATURES = frozenset({'dep:iroh-blobs'})
METADATA_PREFIXES = (
    'Evidence-ID:',
    'Task-ID:',
    'Artifact-Type:',
    'Covers:',
)


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--default-features', required=True, type=Path)
    parser.add_argument('--smoke-manifest', required=True, type=Path)
    parser.add_argument('--smoke-source', required=True, type=Path)
    parser.add_argument('--cluster-features', required=True, type=Path)
    parser.add_argument('--cli-features', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    return parser.parse_args(argv)


def normalized_path(path: Path) -> Path:
    return path.expanduser().resolve()


def repo_root() -> Path:
    return Path(__file__).resolve().parents[1]


def strip_evidence_metadata(text: str) -> str:
    lines = text.splitlines()
    line_index = 0
    while line_index < len(lines) and any(lines[line_index].startswith(prefix) for prefix in METADATA_PREFIXES):
        line_index += 1
    while line_index < len(lines) and lines[line_index] == '':
        line_index += 1
    remaining_lines = lines[line_index:]
    if not remaining_lines:
        return ''
    return '\n'.join(remaining_lines) + '\n'


def read_evidence_text(path: Path) -> str:
    return strip_evidence_metadata(path.read_text())


def load_toml(path: Path) -> dict[str, Any]:
    return tomllib.loads(read_evidence_text(path))


def default_feature_result(path: Path) -> dict[str, Any]:
    text = read_evidence_text(path)
    offending_lines = [line for line in text.splitlines() if CORE_FEATURE_MARKER in line]
    return {
        'ok': not offending_lines,
        'path': str(path),
        'offending_lines': offending_lines,
    }


def manifest_feature_result(path: Path, expected_feature_names: frozenset[str], feature_requirements: dict[str, frozenset[str]]) -> dict[str, Any]:
    data = load_toml(path)
    features = data.get('features', {})
    default_features = features.get('default', None)
    actual_feature_names = frozenset(str(name) for name in features.keys())
    messages: list[str] = []
    if default_features != EMPTY_LIST:
        messages.append(f'default features expected [], found {default_features!r}')
    if actual_feature_names != expected_feature_names:
        messages.append(
            f'feature names expected {sorted(expected_feature_names)!r}, found {sorted(actual_feature_names)!r}'
        )
    for feature_name, required_entries in feature_requirements.items():
        actual_entries = frozenset(str(entry) for entry in features.get(feature_name, []))
        if actual_entries != required_entries:
            messages.append(
                f'{feature_name} expected {sorted(required_entries)!r}, found {sorted(actual_entries)!r}'
            )
    if 'std' in actual_feature_names:
        messages.append('std feature must not exist')
    return {'ok': not messages, 'path': str(path), 'messages': messages}


def core_manifest_result(path: Path) -> dict[str, Any]:
    return manifest_feature_result(
        path,
        expected_feature_names=CORE_DEFAULT_FEATURES,
        feature_requirements={'sql': CORE_SQL_FEATURES},
    )


def shell_manifest_result(path: Path) -> dict[str, Any]:
    return manifest_feature_result(
        path,
        expected_feature_names=SHELL_DEFAULT_FEATURES,
        feature_requirements={
            'sql': SHELL_SQL_FEATURES,
            'layer': SHELL_LAYER_FEATURES,
            'global-discovery': SHELL_GLOBAL_DISCOVERY_FEATURES,
        },
    )


def smoke_manifest_result(path: Path) -> dict[str, Any]:
    data = load_toml(path)
    dependency = data.get('dependencies', {}).get('aspen-core')
    messages: list[str] = []
    if dependency is None:
        messages.append('missing aspen-core dependency')
    elif isinstance(dependency, dict):
        if 'features' in dependency:
            messages.append('aspen-core dependency must not override features')
        if 'default-features' in dependency:
            messages.append('aspen-core dependency must use bare default resolution')
        if 'package' in dependency:
            messages.append('alloc smoke consumer must depend on real aspen-core package, not shell alias')
    else:
        messages.append('aspen-core dependency should be an explicit table entry')
    return {'ok': not messages, 'path': str(path), 'messages': messages}


def smoke_source_result(path: Path) -> dict[str, Any]:
    text = read_evidence_text(path)
    messages: list[str] = []
    if NO_STD_MARKER not in text:
        messages.append('missing #![no_std]')
    if ALLOC_MARKER not in text:
        messages.append('missing extern crate alloc')
    if ASPEN_CORE_MARKER not in text:
        messages.append('missing explicit aspen_core usage')
    return {'ok': not messages, 'path': str(path), 'messages': messages}


def consumer_feature_result(path: Path, required_markers: list[str]) -> dict[str, Any]:
    text = read_evidence_text(path)
    missing_markers = [marker for marker in required_markers if marker not in text]
    return {'ok': not missing_markers, 'path': str(path), 'missing_markers': missing_markers}


def write_output(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + '\n')


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    root = repo_root()
    core_manifest_path = root / 'crates/aspen-core/Cargo.toml'
    shell_manifest_path = root / 'crates/aspen-core-shell/Cargo.toml'
    results = {
        'default_features': default_feature_result(normalized_path(args.default_features)),
        'core_manifest': core_manifest_result(core_manifest_path),
        'shell_manifest': shell_manifest_result(shell_manifest_path),
        'smoke_manifest': smoke_manifest_result(normalized_path(args.smoke_manifest)),
        'smoke_source': smoke_source_result(normalized_path(args.smoke_source)),
        'cluster_features': consumer_feature_result(
            normalized_path(args.cluster_features),
            [SHELL_DEFAULT_FEATURE_MARKER],
        ),
        'cli_features': consumer_feature_result(
            normalized_path(args.cli_features),
            [SHELL_DEFAULT_FEATURE_MARKER, SHELL_LAYER_FEATURE_MARKER],
        ),
    }
    failures = [name for name, result in results.items() if not result['ok']]
    payload = {
        'ok': not failures,
        'failures': failures,
        'results': results,
    }
    write_output(normalized_path(args.output), payload)
    if failures:
        print('feature claims check failed', file=sys.stderr)
        for name in failures:
            print(f'- {name}', file=sys.stderr)
        return 1
    return 0


if __name__ == '__main__':
    raise SystemExit(main(sys.argv[1:]))
