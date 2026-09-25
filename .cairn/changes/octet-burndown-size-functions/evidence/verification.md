# Verification: Octet burn-down, functions within the 70-line limit

Base: `8fbab9887` (C3b sync/archive) on `integration/stack-20260925` (`70a9f0b53`). Octet: pinned `octet-toolchain`
`fc38f593`. Private `CARGO_TARGET_DIR`; `nice -n 10`, `CARGO_BUILD_JOBS=16`, Nix `--max-jobs 2`. Cairn: the installed
store binary `/nix/store/z1a7pddlrs9myzyypm10v8ljd0ipfkhd-cairn-0.1.0/bin/cairn`, run with the explicit
`cairn-policy/generated/cairn-policy.json` policy.

## Octet

| Scope | Base findings | After | `function_length` findings (sites) |
|---|---:|---:|---:|
| workspace (`cargo octet check`) | 2692 | 2499 | 202 (127) → 0 |
| `-p molten --lib` | 1111 | 1046 | 70 (70) → 0 |

Per-lint delta (`octet-lint-diff.txt`):
- `function_length`: 202 → 0 workspace, 70 → 0 lib.
- `path_segment_repetition`: workspace 1877 → 1886 findings and 1038 → 1042 sites. The lib goes 794 → 799. Of that:
  - +5 items, all unmasked and none new (details below).
  - −1 site: `NegativeFaultFixture` moved out of the test function `direct_negative_fault_fixtures_deny_before_side_effects`, whose name contained `negative`, into its module.
- Every other family is unchanged. The `excessive_file_length` site set is identical file for file.

### Unmasked pre-existing names (C2 inputs)

`unmasked-path-segment-items.txt` shows the following for each item:
- The item path and signature are byte-identical at `8fbab9887`.
- The base item snippet contains an Octet `COMPATIBILITY_WORDS` keyword.
- The candidate snippet contains none.

Octet's `has_compatibility_documentation` (`src/naming/path_segment_repetition.rs:128`, called at `:171`, pinned
`fc38f593`) exempts any item whose source snippet contains `compat`, `compatibility`, `external`, `mandated`, or `api`.
Extracting helpers moved those words out of the five bodies.

| Item | Base keyword text |
|---|---|
| `content_store_adapter::content_store_port_descriptors` | `determinism: DeterminismClass::ExternalEffect,` |
| `fabric_membership::fabric_membership_port_descriptors` | `crate::fabric::DeterminismClass::ExternalEffect,` |
| `fabric_observability::fabric_observability_port_descriptors` | `determinism: crate::fabric::DeterminismClass::ExternalEffect,` |
| `fabric_time::fabric_time_port_descriptors` | `(crate::fabric::DeterminismClass::ExternalEffect, crate::fabric::ReplayClass::RecordedEffectRequired)` |
| `system_extension::run_executable_system_extension_fixture` | `rollback_input.compatible_state_schemas = vec![…];` |

These five are renames for the C2 naming decision. They are not renamed here, and no keyword is added to exempt them
again.

### Growth found and fixed during the change

- The first candidate run named 15 new private helpers with ancestor segments, for example `HarnessPlan` under
  `cluster_harness`. All of them were renamed.
- Ten `use super::command::*Input` imports gave 20 `non_trait_imports` findings. They were replaced with qualified
  paths.
- Three files crossed the 300-line limit. The fixes moved the assembly test to `raft/iroh/bundle.rs`,
  `codec_checked_manifest_ref` to the next chunk-store include body, and the optimization-cap comparison to
  `wasm::performance::optimization`.

## Rust gates

- `cargo fmt --check`: exit 0. `cargo clippy --workspace --all-targets -- -D warnings`: exit 0.
- `cargo test --workspace --no-fail-fast`: exit 0, 2091 passed, 0 failed, 0 ignored.
- `cargo test --test fabricboundarycompat`: 5 of 5 passed. `cargo test --lib fabric_execution::`: 10 of 10 passed.
- The diff adds no `allow`. `dylint.toml`, baselines, and quarantine files are unchanged, and no `as` cast is added.
- No Octet exemption keyword (`compat`, `external`, `mandated`, or `api`) is added to exempt an item. Only two new
  helpers carry such words, both from moved identifiers:
  - `testing::distributed::adoption_digest` returns `ExternalDigestMappingReceipt`.
  - `world_snapshot::service::restored_receipt` takes `compatibility_ref`.

  Neither name repeats an ancestor segment, so neither keyword hides a finding. The `path_segment_repetition` item diff
  shows nothing disappearing other than `NegativeFaultFixture`.

## Byte identity against the base binary (`8fbab9887`)

- `fixture-receipt-comparison.txt`: all 6 `examples/*.preserves` harness suites give identical exit codes and report
  and gate-receipt BLAKE3 hashes.
- `simulation-fixture-comparison.txt`:
  - `molten fabric-simulation run` (100 files) and `shrink` (3 files) produce identical output trees and stdout.
  - All 482 recursive `molten … --help` pages are identical. This covers the CLI variants moved into `clap::Args`
    structs.

## Flake checks

`flake-checks.txt` lists the 22 touched-surface checks built with `nix build .#checks.x86_64-linux.<name>`. All
exit 0:
- The `addressable-actor`, `coordination-delivery`, `fabric-execution`, `native-system-extension`, `prolly-map`,
  `world-head`, `world-snapshot`, and `world-promotion` `*-octet-deny-all` checks.
- The `addressable-actor`, `content-store-adapter`, `coordination-delivery`, `fabric-cryptographic-identity`,
  `fabric-execution`, `fabric-membership-placement`, `fabric-observability`, `native-system-extension-host`,
  `wasm-component`, and `wasm-component-performance` profiles.
- `fabric-port-boundaries`, `materialization-authority`, `release-profile-validation`, and
  `requirement-traceability-gate`.

## Review and lifecycle

The no-spec plan review approves the proposal, design, acceptance, and tasks with no findings. The proposal, design,
and tasks gates pass, and `cairn validate --strict` reports no issues.
