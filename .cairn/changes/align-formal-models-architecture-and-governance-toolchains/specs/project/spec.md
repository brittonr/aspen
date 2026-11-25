# Project deltas

## ADDED Requirements

### Requirement: Modularity uses real crate and module encapsulation rather than include-spliced shards
r[molten.project.real_module_encapsulation] Aspen MUST organize subsystems into real encapsulated modules and workspace crates with single canonical names rather than splicing 300-line `parts/**/body.rs` fragments via `include!()` or maintaining triple-aliased `#[doc(hidden)]` facade pseudonyms. Production Rust MUST NOT `include_str!` generated config JSON from `docs/`, and unwired boundary planner stubs in `crates/molten-core` MUST either be wired into production paths or removed.

#### Scenario: Runtime config embeds from config directory only
- GIVEN a production Rust module embedding generated Nickel JSON
- WHEN the repository hygiene check scans `include_str!` paths
- THEN any reference into `docs/` MUST fail the check.

#### Scenario: Documentation matches actual crate dependencies
- GIVEN `docs/modularity-boundaries.md` describes `crates/molten-core` dependencies
- WHEN dependency boundaries are audited against `crates/molten-core/Cargo.toml`
- THEN the document MUST accurately enumerate external dependencies rather than claiming std-only.

### Requirement: Governance toolchains use single pinned revisions without stale backups or untracked discrepancy disclaimers
r[molten.project.unified_governance_pins_and_defect_tracking] `flake.nix`, `flake.lock`, and `Cargo.toml` MUST converge on single reviewed pins for Cairn, Octet, and ChaosControl (without placeholder hashes in `cairn-policy/default.ncl` or tracked `.stale-bak` files), demote unused external research pilots out of default CI, and require every source-vs-spec discrepancy noted in `docs/technical/` to link to a tracked Cairn change.

#### Scenario: Duplicate governance pins or stale backup fails hygiene check
- GIVEN a tracked `.stale-bak` file or placeholder policy hash in `cairn-policy/`
- WHEN repository hygiene checks run
- THEN the check MUST fail closed.

#### Scenario: Source discrepancy links to tracked defect change
- GIVEN a technical companion article documents a behavioral discrepancy between code and governing spec
- WHEN documentation review runs
- THEN the discrepancy MUST link to an active Cairn defect change rather than standing alone as an untracked non-claim.
