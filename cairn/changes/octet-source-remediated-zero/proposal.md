## Why

Molten's Octet gate currently passes in a configuration-clean state, but the documented caveat remains: several broad lint families are disabled in `dylint.toml` while source-remediated-zero work is still outstanding. The largest visible hotspot is the monolithic CLI shell in `src/main.rs`, where command parsing and dispatch for many domains live in one file.

Strict release evidence should eventually distinguish a true source-shaped zero from a configuration-clean pass. This change starts the source-remediated-zero burn-down by extracting the Octet CLI command group into a focused shell module, preserving canonical receipt behavior while reducing the main CLI surface.

## What Changes

- Move Octet command enums and dispatch from `src/main.rs` into a focused `src/cli_octet.rs` module.
- Preserve existing `molten test octet ...` command syntax, receipt output, denial behavior, and canonical Preserves values.
- Track the remaining disabled lint family burn-down as explicit future work rather than claiming the full source-remediated-zero state is complete.
- Require focused validation and refreshed Octet evidence before claiming source-gate improvements.

## Impact

This is a low-risk first vertical slice in the source-remediated-zero roadmap. It reduces the monolithic imperative CLI shell without changing runtime semantics or release evidence contracts. Future slices should continue splitting CLI groups and high-value modules, then remove or narrow the disabled lint family caveats when the source shape supports it.
