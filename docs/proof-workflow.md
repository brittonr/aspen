# Molten proof workflow

Molten proof-affecting changes use canonical receipts as normative evidence and rendered summaries only as review aids. This page records the proof checklist, receipt-backed traceability path, layered proof boundaries, deny-path matrix, and readback workflow.

## Proof checklist for Cairn changes

Use this checklist for changes that alter proof, gates, evidence, traceability, replay, release, or mutation behavior:

- Proof claim: what the change proves.
- Out-of-scope claims: what the change explicitly does not prove.
- Trusted assumptions: policy, fixture, toolchain, or review inputs that remain assumed.
- Positive evidence: pass receipts, tests, Hegel RS properties, or fixtures.
- Negative evidence: expected-deny receipts, stale/tamper cases, no-mutation evidence, or explicit exemption.
- Canonical refs: verification-run, gate, aggregate proof, or release receipt refs.
- Traceability updates: positive and negative coverage entries or receipt-backed inputs.
- Hegel RS properties: required when pure core invariants change and generated inputs add value beyond examples.
- Regeneration commands: the smallest command set that recreates the evidence.

Documentation-only, operator-guidance, and non-executable changes may record an explicit exemption with supporting doc evidence. Checklist items should map to Cairn tasks or evidence notes so incomplete proof work stays visible.

## Verification-run receipts

`verification-run-receipt-v1` records requirement id, positive/negative coverage kind, target, argv, profile ref, toolchain refs, exit status, output refs, produced artifact refs, diagnostics, and decision. Logs remain non-normative.

Example receipt emission from explicit refs:

```sh
molten test traceability verification-run \
  --requirement molten.testing.receipt_driven_traceability.coverage_derivation \
  --coverage-kind positive \
  --target src/testing/traceability.rs \
  --argv cargo --argv test --argv trace_core::tests \
  --profile-ref blake3:profile \
  --toolchain-ref blake3:toolchain \
  --exit-status 0 \
  --stdout-ref blake3:stdout \
  --stderr-ref blake3:stderr \
  --artifact-ref blake3:traceability-manifest \
  --out target/proof/coverage-positive.preserves
```

Positive coverage expects a passing run. Negative coverage expects an expected-deny run. A wrong exit status, malformed ref, stale target, duplicate receipt, or wrong requirement/kind keeps traceability fail-closed.

## Receipt-backed traceability

Raw coverage tuples remain compatibility-only and are labeled in summaries. Release profiles can require receipt-backed coverage:

```sh
molten test traceability scan --root . --changed-only \
  --receipt target/proof/coverage-positive.preserves \
  --receipt target/proof/coverage-negative.preserves \
  --require-receipt-backed \
  --out target/proof/traceability.preserves \
  --summary-out target/proof/traceability-summary.txt \
  --readback-out target/proof/proof-readback.txt
```

The pure core derives coverage from validated receipt fields rather than rendered logs. The Nix/release gate surface can call the same scan command and fail closed on missing positive, missing negative, stale receipt refs, duplicate receipt refs, or compatibility-only coverage when receipt-backed coverage is required. `tests/evidence-matrix.ncl` is the checked-in Nickel matrix source for durable review; its exported entries feed the same requirement ids, positive/negative coverage split, artifact refs, receipt refs, evidence scope, and diagnostic-only exemptions.

## Testing hardening receipts

`molten::testing_hardening` provides pure in-memory builders for the current testing-harness hardening rails:

- `boundary-coverage-gate-v1` records required and observed positive/negative runtime boundary classes, explicit exemptions, missing classes, and stale evidence refs.
- `ci-test-run-receipt-v1` binds source marker, nextest profile, command surface, nextest config, Cargo metadata, binaries metadata, JUnit ref, counts, decision, diagnostics, and caveats; JUnit alone is only a rendered view.
- `tamper-negative-matrix-v1` lists positive controls plus generated negative mutations such as stale refs, wrong kinds, duplicate members, noncanonical values, diagnostic-only pass misuse, and unsupported schema versions.
- `hegel-counterexample-fixture-v1` binds property id, generator profile, seed, shrink path, shrunk input ref, replay identity, traces, receipts, diagnostics, and confidentiality handling before promotion.
- `replay-smoke-gate-v1` compares fresh run, replay, and fresh-rerun canonical refs, while live-only, exploratory, unavailable, or diagnostic-only suites remain visibly excluded from deterministic pass evidence.
- `nextest-profile-matrix-v1` records semantic profile ids, command surfaces, filter expressions, expected artifacts/JUnit paths, retry policy, excluded non-replayable partitions, cost class, evidence scope, platform availability, diagnostics, and release caveats. Deterministic profiles must exclude live-only, VM-only, exploratory, retry-only, and diagnostic-only partitions; VM, dogfood, and exploratory profiles remain platform-scoped or diagnostic evidence only.
- `cli-receipt-first-gate-v1` records whether evidence-bearing CLI tests asserted canonical artifacts or receipts before relying on stdout, stderr, JUnit, markdown, JSON, or terminal summaries.
- `effective-config-readback-v1` records normalized effective values, field-local source traces, profile refs, override refs, default caveats, diagnostics, and canonical BLAKE3 identity. It is readback evidence only; install/run/delete/retention/live-send/policy gates still require their subsystem receipts and authority.
- `context-profile-expansion-v1` records operator context profile refs, operation scope, expanded policy/authority/resource/evidence/retention refs, additive evidence overrides, and denial diagnostics for unsupported scopes or conflicting overrides. Context profiles are convenience evidence only; downstream gates evaluate the expanded refs.
- `release-profile-validation-v1` records development/pilot/release tier checks, non-placeholder release refs, stack-provenance-required status, accepted Valence policy hashes, generated-export freshness, and evidence-only non-claims.
- `node-profile-config-resolution-v1` records checked profile metadata, profile-backed effective node config refs, accepted/denied override diagnostics, local-fixture caveats, and startup-bound profile metadata refs.
- `claim-subject-selector-v1`, `authority-claim-v1`, and `authority-claim-admission-v1` record claim-scoped external authority over explicit selectors, `claim:attest` capability requests, UCAN/Basalt/capability proof refs, and non-authority discovery/transport diagnostics.
- `sans-io-protocol-transition-v1`, `sans-io-shell-drain-v1`, and `sans-io-replay-fixture-v1` record deterministic protocol-core transitions, pre-admission shell-drain gates, and replayable protocol input/output refs without live IO.

Focused checks: `cargo test hardening --lib`, `cargo test config_portability --lib`, `cargo test effective_config --lib`, `cargo test context_profile --lib`, `cargo test release_profile --lib`, `cargo test node_profile_config --lib`, `cargo test claim_authority --lib`, `cargo test sans_io --lib`, `cargo test local_node_init_run_status_stop_and_restart_recovery_are_receipted --lib`, `cargo test --test cliharness ci_run_receipt`, `cargo test --test cliharness effective_config`, `cargo test --test cliharness context_profile`, `cargo test --test cliharness cli_node_profile_backed_init_binds_startup_metadata`, `molten test traceability config-lint --root . --out target/config-portability.preserves --summary-out target/config-portability.txt`, `nickel export tests/evidence-matrix.ncl`, `nix build .#checks.x86_64-linux.contract-export-drift-gate --no-link`, and `nix build .#checks.x86_64-linux.nextest-config`.

## Aggregate proof obligations

Broad workflow claims should decompose into child obligations:

- `input-validation`
- `canonicalization`
- `admission`
- `mutation-boundary`
- `replay-determinism`
- `fail-closed-negative`

Aggregate proof manifests pass only when every required child is present, scoped to the subject, bound to valid prerequisite/receipt refs, and has the expected decision. Negative obligations use expected-deny receipts. Aggregate manifests remain evidence-only and do not replace subsystem gates.

## Layered proof contract

Layer roles are:

- pure-core proof
- gate proof
- replay proof
- release proof
- operator readback

Each layer binds lower layers by explicit ids/refs. Validation denies stale child refs, cycles, wrong-subject links, unsupported role links, denied children, and diagnostic/readback layers used as pass evidence. Higher layers do not promote trust automatically.

## Deny-path matrix

Proof-bearing gates should declare required negative classes:

- missing artifact
- stale ref
- malformed schema
- wrong signer
- wrong purpose
- tampered bytes
- duplicate receipt
- denied mutation
- diagnostic-only not pass

Denials before side effects must bind unchanged before/after state refs or a no-mutation receipt. Logs are diagnostic-only.

## Operator readbacks

Proof readbacks group requirements with positive receipts, negative receipts, artifact refs, diagnostics, and caveats. They are deterministic rendered views over canonical receipts. They do not grant authority, policy, provenance, resource, transport, source-gate, retention, destructive-operation trust, or permission to bypass subsystem gates.
