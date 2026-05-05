# Typed Nickel Contract Registry

This registry records the source-of-truth decision for Aspen's typed Nickel contract families. It is paired with the machine-readable Nickel registry at `schemas/typed-nickel-contract-registry.ncl` and the checker at `scripts/check-typed-nickel-contract-registry.py`.

## Source-of-truth classes

- `rust-derived`: Rust owns the canonical serialized DTO/evidence/protocol shape. Nickel contracts are generated from Rust schema metadata and checked for freshness.
- `nickel-authored`: Nickel owns the human-facing configuration/policy contract. Rust consumes only validated exported values.
- `non-candidate`: The surface must not move into Nickel; Nickel may only validate adjacent data references when explicitly listed elsewhere.

## Inventory summary

| Family | Class | Owner source | Artifact/status |
| --- | --- | --- | --- |
| CI pipeline config | `nickel-authored` | `crates/aspen-ci/src/config/schema/ci_schema.ncl` | Existing embedded Nickel schema; needs continued typecheck/config tests. |
| Deploy protocol DTOs | `rust-derived` | `crates/aspen-ci/src/orchestrator/deploy_executor.rs` | Generated `schemas/deploy-protocol.ncl`; snapshot/freshness checked by `cargo test -p aspen-ci test_deploy_protocol_schema_snapshot` and Nickel typechecked. |
| Dogfood run receipt | `rust-derived` | `crates/aspen-dogfood/src/receipt.rs` | Generated `schemas/dogfood-run-receipt.ncl`; freshness checked by `scripts/generate-typed-nickel-contracts.py --check`. |
| Native CI run receipt | `rust-derived` | `crates/aspen-client-api/src/messages/ci.rs` | Generated `schemas/ci-run-receipt.ncl`; freshness checked by `scripts/generate-typed-nickel-contracts.py --check`. |
| Node/cluster/profile config | `nickel-authored` | `crates/aspen-nickel/src/schema/node_config.ncl` | Existing schema; needs profile/feature/trust hardening. |
| Test harness suite manifests | `nickel-authored` | `test-harness/schema.ncl` | Existing schema + generated inventory freshness gate. |
| Patchbay fault scenarios | `nickel-authored` | `test-harness/suites/patchbay/patchbay-fault.ncl` | Existing suite manifests; needs bounded fault-dimension contracts. |
| Crate-extraction readiness policy | `nickel-authored` | `docs/crate-extraction/policy.ncl` | Existing policy; needs deeper readiness/evidence contracts. |
| Feature bundle policy | `nickel-authored` | `Cargo.toml` / release profile policy | Planned `schemas/feature-bundles.ncl`. |
| Snix build executor policy | `nickel-authored` | `crates/aspen-cluster/src/config/snix.rs` | Planned `schemas/snix-build-executor-policy.ncl`. |
| Trust/bootstrap policy | `nickel-authored` | `docs/trust-quorum.md` | Planned `schemas/trust-bootstrap-policy.ncl`. |
| Operator diagnostics evidence | `rust-derived` | promoted Rust diagnostic/receipt DTOs | Planned generated evidence contracts. |

## Crunch prior-art classification

Reusable items from `../crunch/crunch` are classified before implementation:

| Crunch source | Classification | Aspen use |
| --- | --- | --- |
| `lib/contracts.ncl` | `adapt` | Generic enum/string/path/range helper idioms for Aspen-specific contracts. |
| `lib/project.ncl` | `adapt` | Profile/default composition patterns, not Crunch project semantics. |
| `lib/project_outputs.ncl` | `adapt` | CI artifact/deploy output selection patterns, not Crunch output semantics. |
| `builders/mk_derivation.ncl` | `reject` | Crunch derivation/runtime builder semantics are not Aspen policy. |
| `lib/inventory.ncl` | `adapt` | Suite/cluster inventory topology shape with Aspen field names. |
| `lib/system_module.ncl` | `adapt` | Node/profile module topology and defaults. |
| `crates/crunch-glue/src/types.rs` | `adapt` | Rust DTO boundary conventions for Nickel-authored exports. |
| `crates/crunch-project-core/src/manifest.rs` | `adapt` | Version/default/bounds/duplicate-reference validation patterns. |
| `src/build_report.rs` | `adapt` | Evidence/report DTO organization for Aspen receipts. |
| `src/operator_diagnostics.rs` | `adapt` | Diagnostic report structure with Aspen redaction/ownership rules. |
| `src/witness_rebuild.rs` | `reject` | Witness/rebuild workflow semantics are Crunch-specific. |
| `crates/crunch-attestation-core/src/schema.rs` | `adapt` | Attestation schema versioning/evidence DTO patterns. |

## Explicit non-candidates

The following remain Rust-owned or out-of-scope for Nickel contracts:

- Raft state transitions and distributed behavior.
- Protocol enum discriminant ownership and strict wire ABI compatibility.
- Cryptographic internals: token verification, key derivation, HMACs, Shamir shares, and algorithm choices.
- Raw secret material: bearer tokens, private keys, generated secret values, or credentials.
- Hot-path runtime constants and Tiger Style resource bounds that require Rust compile-time ownership.
- Crunch-owned derivation, store hashing, witness, and runtime build semantics.

## Validation

Run:

```bash
nix run nixpkgs#nickel -- typecheck schemas/deploy-protocol.ncl
cargo test -p aspen-ci test_deploy_protocol_schema_snapshot
python3 scripts/check-typed-nickel-contract-registry.py
python3 scripts/generate-typed-nickel-contracts.py --check
nix run nixpkgs#nickel -- typecheck schemas/dogfood-run-receipt.ncl
nix run nixpkgs#nickel -- typecheck schemas/ci-run-receipt.ncl
openspec validate type-nickel-contract-boundaries --strict --json
git diff --check
```

If `nickel` is not installed on `PATH`, the checker tries `nix run nixpkgs#nickel -- export --format json ...`.
