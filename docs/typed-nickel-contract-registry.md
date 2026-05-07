# Typed Nickel Contract Registry

This registry records the source-of-truth decision for Aspen's typed Nickel contract families. It is paired with the machine-readable Nickel registry at `schemas/typed-nickel-contract-registry.ncl` and the checker at `scripts/check-typed-nickel-contract-registry.py`.

## Source-of-truth classes

- `rust-derived`: Rust owns the canonical serialized DTO/evidence/protocol shape. Nickel contracts are generated from Rust schema metadata and checked for freshness.
- `nickel-authored`: Nickel owns the human-facing configuration/policy contract. Rust consumes only validated exported values.
- `non-candidate`: The surface must not move into Nickel; Nickel may only validate adjacent data references when explicitly listed elsewhere.

## Inventory summary

| Family | Class | Owner source | Artifact/status |
| --- | --- | --- | --- |
| CI pipeline config | `nickel-authored` | `crates/aspen-ci/src/config/schema/ci_schema.ncl` | Existing embedded Nickel schema expanded with typed cache, artifact, deploy, retry/timeout, and validation-only policy contracts. |
| Deploy protocol DTOs | `rust-derived` | `crates/aspen-ci/src/orchestrator/deploy_executor.rs` | Generated `schemas/deploy-protocol.ncl`; snapshot/freshness checked by `cargo test -p aspen-ci test_deploy_protocol_schema_snapshot` and Nickel typechecked. |
| Dogfood run receipt | `rust-derived` | `crates/aspen-dogfood/src/receipt.rs` | Generated `schemas/dogfood-run-receipt.ncl`; freshness checked by `scripts/generate-typed-nickel-contracts.py --check`. |
| Native CI run receipt | `rust-derived` | `crates/aspen-client-api/src/messages/ci.rs` | Generated `schemas/ci-run-receipt.ncl`; freshness checked by `scripts/generate-typed-nickel-contracts.py --check`. |
| Node/cluster/profile config | `nickel-authored` | `crates/aspen-nickel/src/schema/node_config.ncl` | Existing schema expanded with typed bootstrap peers, feature bundle refs, metrics/OTLP, and trust references. |
| Test harness suite manifests | `nickel-authored` | `test-harness/schema.ncl` | Existing schema + generated inventory freshness gate, expanded with capabilities, isolation assumptions, timeout classes, expected artifacts, faults, and convergence assertions. |
| Patchbay fault scenarios | `nickel-authored` | `test-harness/suites/patchbay/patchbay-fault.ncl` | Existing suite manifests covered by shared bounded fault-dimension contracts. |
| Crate-extraction readiness policy | `nickel-authored` | `docs/crate-extraction/policy.ncl` | Existing policy deepened with readiness/class enums, publication metadata, and required evidence contracts. |
| Feature bundle policy | `nickel-authored` | `schemas/feature-bundles.ncl` | Existing profile/bundle contract for minimal, dogfood, CI worker, Forge, snix, and full profiles. |
| Snix build executor policy | `nickel-authored` | `schemas/snix-build-executor-policy.ncl` | Existing bounded sandbox/cache/fallback policy contract; build behavior remains Rust-owned. |
| Trust/bootstrap policy | `nickel-authored` | `schemas/trust-bootstrap-policy.ncl` | Existing secret-free quorum/bootstrap policy contract with raw-secret rejection fixtures. |
| Operator diagnostics evidence | `rust-derived` | `schemas/operator-diagnostics-evidence.ncl` | Existing common diagnostic-envelope boundary contract; promoted DTO-specific contracts remain Rust-derived follow-up. |
| Sponsored runtime policy | `nickel-authored` | `schemas/sponsored-runtime-policy.ncl` | Provider offers, sponsor policies, resource class catalogs, and admission profiles; Rust owns admission behavior, quota ledgers, and usage receipts. |
| Sponsored runtime grant | `rust-derived` | `crates/aspen-runtime-core/src/lib.rs` | Generated `schemas/sponsored-runtime-grant.ncl`; Rust owns grant semantics and authorization checks. |
| Sponsored quota ledger | `rust-derived` | `crates/aspen-runtime-core/src/lib.rs` | Generated `schemas/sponsored-quota-ledger.ncl`; Rust owns quota arithmetic and ledger state transitions. |
| Sponsored usage receipt | `rust-derived` | `crates/aspen-runtime-core/src/lib.rs` | Generated `schemas/sponsored-usage-receipt.ncl`; Rust owns metering/receipt emission and Nickel validates evidence shape. |

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
python3 scripts/check-typed-nickel-contract-fixtures.py
python3 scripts/check-typed-nickel-contract-registry.py
python3 scripts/generate-typed-nickel-contracts.py --check
nix run nixpkgs#nickel -- typecheck schemas/deploy-protocol.ncl
nix run nixpkgs#nickel -- typecheck schemas/sponsored-runtime-policy.ncl
nix run nixpkgs#nickel -- typecheck schemas/sponsored-runtime-grant.ncl
nix run nixpkgs#nickel -- typecheck schemas/sponsored-quota-ledger.ncl
nix run nixpkgs#nickel -- typecheck schemas/sponsored-usage-receipt.ncl
cargo test -p aspen-ci test_deploy_protocol_schema_snapshot
cargo nextest run -p aspen-ci test_deploy_protocol_schema_snapshot
cargo test -p aspen-client-api ci_receipt_schema_and_status_labels_are_documented --features ci
openspec validate type-nickel-contract-boundaries --strict --json
git diff --check
```

If `nickel` is not installed on `PATH`, the checker tries `nix run nixpkgs#nickel -- export --format json ...`.
