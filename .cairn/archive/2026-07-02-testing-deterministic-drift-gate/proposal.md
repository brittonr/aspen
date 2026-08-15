## Why

Molten’s evidence model depends on deterministic refs: the same declared inputs should produce the same canonical reports, receipts, and release evidence. Existing checks prove a workflow can pass once, but they do not systematically catch drift caused by ambient state, hidden clocks, map order, nondeterministic fixture data, or unstable rendered output leaking into canonical evidence.

## What Changes

- Add an explicit deterministic drift gate for evidence-bearing workflows.
- Run selected workflows in fresh isolated state roots and compare canonical BLAKE3 refs.
- Require all volatile fields to be declared, justified, and excluded only through canonical normalization rules.
- Add fail-closed diagnostics for unexplained drift, ambient state use, unstable refs, and mismatched volatile-field declarations.
- Cover dogfood/repro/release evidence paths first, then extend to VM child evidence where a deterministic profile exists.

## Impact

- **Files**: deterministic comparison core, dogfood/repro test helpers, Nix check or app, negative fixtures, docs/README testing section.
- **Testing**: positive same-input/same-ref fixtures, negative drift injection fixtures, and an explicit gate over selected release-evidence workflows.
