# Evidence gates deltas

## ADDED Requirements

### Requirement: Workspace Octet and strict Cairn validation run hermetically in flake checks
r[molten.evidence_gates.ci_octet_and_cairn_hermetic] `nix flake check` MUST execute workspace-wide Octet checks covering every workspace member (`molten`, `molten-core`, `molten-node-host`, `molten-release-policy`) and MUST execute strict `cairn validate` against a pinned Cairn input and matching policy export rather than relying on unpinned sibling checkouts or local-only hooks.

#### Scenario: Violation in molten-core fails workspace Octet
- GIVEN an Octet rule violation introduced in `crates/molten-core`
- WHEN `cargo octet check` or `nix flake check` runs
- THEN the workspace Octet check MUST fail rather than excluding `molten-core`.

#### Scenario: Strict Cairn policy violation fails flake check
- GIVEN a malformed Cairn change or policy violation in `.cairn/changes/`
- WHEN `nix flake check` runs
- THEN `checks.cairn-validate-strict` MUST fail using the pinned Cairn input without reading `../cairn`.
