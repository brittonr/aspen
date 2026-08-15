## Context

Path-shape findings often come from mechanical repetition in module names, helper names, or deeply qualified call sites. They should be reduced without destabilizing public names or conflating the work with broader imports and size splits.

## Design

### Hotspot workflow

Start from a fresh no-disabled probe and choose one path-shape hotspot. Prefer private aliases, private helper renames, or module-local ownership changes. Public Rust paths, CLI subcommands, receipt labels, and schema names must stay stable unless a separate compatibility change admits the rename.

### Boundary with import hygiene

Path-shape cleanup may use focused aliases, but broad import normalization belongs to the import-hygiene package. If a path-shape fix materially increases import warnings, record that tradeoff and keep the import caveat visible.

### Evidence

Record before/after `path_segment_repetition` counts and note any public path intentionally preserved despite repetition.

## Validation

Run focused tests for the touched domain, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and a no-disabled Octet probe.

## Non-goals

- Do not rename public commands, receipt schemas, or canonical labels for lint counts.
- Do not hide path warnings with suppressions.
- Do not combine unrelated large size-shape splits unless required by the chosen hotspot.
