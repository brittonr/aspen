# Aspen development reference

<a id="development"></a>

Run commands from the repository root after resolving the [development environment prerequisites](handbook/contributing/diagnosing-development-environment-failures.md). See [Contributing](../CONTRIBUTING.md) for the current integration workflow. The command examples retain the `molten` development executable and package names.

Optional cross-process function profiling is documented in [`docs/development-function-profiling.md`](../docs/development-function-profiling.md). Traces are bounded development observations and never release evidence.

Proof-affecting changes should follow [`docs/proof-workflow.md`](../docs/proof-workflow.md) for checklists, receipt-backed traceability, deny-path evidence, layered proof boundaries, and readbacks.

```sh
nix develop
cargo nextest run
cargo test # fallback
```

Nextest profiles:

```sh
cargo nextest run --profile fast-core
cargo nextest run --profile harness
cargo nextest run --profile cli
cargo nextest run --profile distributed-simulation
cargo nextest run --profile vm-platform
cargo nextest run --profile dogfood-soak
cargo nextest run --profile ci
cargo nextest run --profile deterministic
cargo nextest run --profile exploratory
```

The profiles fail flaky tests in CI/deterministic mode, bound hangs with explicit global/slow/leak timeouts, and write JUnit evidence under `target/nextest/<profile>/junit.xml` for release readback. `.config/nextest.toml` now gives each semantic profile an explicit `default-filter`, and `nix build .#checks.x86_64-linux.nextest-config` emits `nextest-profile-matrix-v1` readback that binds filters, command surfaces, JUnit paths, retry policy, cost class, evidence scope, platform availability, and diagnostics. `fast-core`, `harness`, `cli`, and `distributed-simulation` are deterministic evidence profiles only when their filters exclude live-only, VM-only, exploratory, retry-only, and diagnostic-only partitions. `vm-platform` and `dogfood-soak` select Rust tests by name; their names do not establish that a VM booted or an operator workflow ran. Evidence covers only the tests and environments actually executed with the required canonical receipts. `exploratory` retry success remains diagnostic-only; none of these profiles replace canonical subsystem receipts, source gates, policy, provenance, authority, transport, resource, retention, deployment, or release gates.

Nix exposes the CI command and runs nextest through flake checks. The hermetic `nextest` check preserves `cargo-metadata.json`, `binaries-metadata.json`, `junit.xml`, and `ci-test-run-receipt.preserves`; JUnit is a rendered view and cannot replace the canonical CI receipt.

```sh
nix run .#nextest-ci
nix build .#checks.x86_64-linux.nextest
nix build .#checks.x86_64-linux.dogfood-local-node
nix build .#checks.x86_64-linux.nixos-vm-multinode
nix build .#checks.x86_64-linux.nextest-config
```

When running the VM check from a sibling checkout workspace, pass explicit local sibling overrides so unit2nix receives cleaned checkout trees instead of falling back to remote fetches:

```sh
nix build .#checks.x86_64-linux.nixos-vm-multinode -L \
  --override-input basalt-src path:../basalt \
  --override-input cairn-src path:../cairn \
  --override-input valence-src path:../valence \
  --override-input onixpkgs path:../onixpkgs
```

The VM rails use `testers.runNixOSTest` and require working KVM/QEMU support. The ordinary `nixos-vm-multinode` rail's unsigned startup-denial result is not positive signed startup. The opt-in `nix run .#nixos-vm-multinode-signed` rail requires independently selected source trust; retain its exact startup, control-loop, restart, shutdown and fault receipts before claiming those operations succeeded. [Issue #16](https://github.com/brittonr/aspen/issues/16) and the [NixOS validation boundary](nixos-service-module.md#validation-boundary) distinguish these rails. Historical VM/soak receipts and authored check definitions do not qualify a new candidate. Missing VM support or unavailable child evidence must fail or report unavailable, never mint pass evidence.

`dogfood-local-node` depends on the hermetic nextest check, runs `molten dogfood local-node` in a temporary state root, preserves `dogfood-report.preserves`, `release-gate.preserves`, `replay-verify.preserves`, `replay-evidence-index.preserves`, `dogfood-summary.txt`, and an `after-nextest.txt` marker, then emits `nix-dogfood-evidence.preserves` plus `nix-dogfood-verify.preserves` to bind the Nix output path, report ref, release-gate ref, replay verify ref, replay evidence index ref, and nextest dependency marker for release review. It also emits `release-evidence-bundle.preserves`, `release-evidence-bundle-verify.preserves`, `release-promotion-gate.preserves`, `release-promotion-gate.signed.preserves`, `release-promotion-summary.preserves`, `release-export-manifest.preserves`, `release-evidence.tar.zst`, and `release-export-verify.preserves`, which bind the complete release review member set, signed keyring currentness, source/Octet/Cairn evidence markers, a final evidence-only promotion decision, a keyring-verified signature over that exact decision subject ref, a compact readback summary, and a deterministic portable evidence archive plus verification receipt. The check imports a local fixture signing key into a ledger-backed signed receipt keyring, signs the Preserves release members, verifies the bundle with `--require-signed-members` plus `--signed-key-ledger`, runs `release-promote` over the resulting graph, signs the promotion receipt with purpose `release-promotion`, and verifies that signed promotion receipt through the same keyring. These artifacts remain evidence-only and do not grant authority, policy, provenance, resource, transport, source-gate, retention, destructive-operation, or deployment trust.

Release bundle commands can be run manually against a realized check output: `molten dogfood release-bundle-export --output-path OUT --out release-evidence-bundle.preserves`, `molten dogfood release-bundle-verify --output-path OUT --bundle release-evidence-bundle.preserves --receipt-out release-evidence-bundle-verify.preserves`, `molten dogfood release-promote --output-path OUT --bundle-verify release-evidence-bundle-verify.preserves --receipt-out release-promotion-gate.preserves --signed-key-ledger KEYRING --source-evidence ... --octet-evidence ... --cairn-evidence ...`, `molten dogfood release-promotion-summary --output-path OUT --out release-promotion-summary.preserves --signed-key-ledger KEYRING`, `molten dogfood release-export --output-path OUT --out release-evidence.tar.zst --manifest-out release-export-manifest.preserves`, and `molten dogfood release-export-verify --bundle release-evidence.tar.zst --receipt-out release-export-verify.preserves`; export verification emits deny receipts for missing manifests, duplicate paths, extra members, stale refs, or tampered member bytes. Signed member and promotion checks use `molten receipts key import --ledger KEYRING --key-id release-key-v1 ...`, `molten receipts sign RECEIPT --out RECEIPT.signed.preserves --purpose release-evidence|release-promotion ...`, and `molten receipts verify-signed --key-ledger KEYRING --key-id release-key-v1 ...`; `release-bundle-verify --require-signed-members --signed-key-ledger KEYRING --signed-member RECEIPT.signed.preserves ...` emits deny receipts for missing, wrong-signer, wrong-purpose, revoked-key, stale, or tampered members instead of treating logs as primary evidence. Keyring management also supports `molten receipts key list/show/revoke/rotate`; currentness and revocation are verification evidence, not authority.

Operator receipt readback is available for local dogfood ledgers: `molten receipts list --ledger STATE/ledger`, `molten receipts show REF --ledger STATE/ledger`, `molten receipts validate REF --ledger STATE/ledger`, and `molten receipts export REF --ledger STATE/ledger --out receipt.preserves`. The commands read canonical Preserves artifacts from the content-addressed ledger, validate supported dogfood/operator receipt kinds, render only non-normative summaries by default, and keep logs auxiliary rather than primary evidence.

For revision-pinned OnixResearch Git dependencies, the flake uses reviewed immutable HTTPS source inputs. Authentication and binding use one unified Artifact input. The default source graph requires no GitHub SSH credentials; private Radicle sources remain explicit public-seed or local-profile prerequisites. Sibling checkouts can be substituted only through explicit development-time `--override-input` paths. The substitution filters generated and VCS-heavy directories such as `target/` and `.git/` before unit2nix sees the source. Release validation compares Cargo manifest, lockfile, Nix, release-policy, and generated-plan identities and rejects local paths as release evidence. See [`docs/reproducible-dependencies.md`](../docs/reproducible-dependencies.md). Historical August internal-pilot nextest output: `/nix/store/d50qlba4am1pb4f59l71djjg1bl6adhi-molten-nextest`.

The historical `0.1.0` limited-pilot candidate was commit `a4f111690b6962f04d9320fd93d09c7dd1ad2fd0`. Its source reference is `blake3:80e3ceb18784504c7573191fce72e121d0789613c6c5f7bdcecbdd9ae0e4cdb7`.

The candidate gate passed as `blake3:9d15e7f99a34ba5f1a979d07cd7023025347144349db50caacca6ddbb90ab1e4`. The pilot decision is `blake3:ea03ddbada910b33a09fe3156e9908b2b7c1f1b0c6b14b39570ae05abdaf27cb`.

See [`docs/releases/molten-0.1.0.md`](../docs/releases/molten-0.1.0.md) and [`release/molten-0.1.0-pilot.ncl`](../release/molten-0.1.0-pilot.ncl) for scope and evidence. Preserved artifacts live with the archived release Cairn. They remain evidence-only and grant no runtime or release authority.

## Strict Octet source-gate sequence

```sh
cargo octet check --artifact-dir target/octet
cargo octet check -p molten --artifact-dir target/octet-lib -- --lib
cargo octet object corpus receipt \
  --output target/octet/object-corpus-receipt.json \
  $(find src -name "*.rs" | sort)
cargo run -- test octet artifacts import \
  --artifacts target/octet \
  --ledger target/octet-ledger \
  --receipt-out target/octet/artifact-ledger-receipt.preserves
cargo run -- test octet gate \
  --artifacts target/octet \
  --profile strict-ci \
  --receipt-out target/octet/gate-receipt.preserves
cargo run -- test octet remediation plan \
  --artifacts target/octet \
  --lib-artifacts target/octet-lib \
  --focused-object-corpus target/octet/object-corpus-receipt.json \
  --receipt-out target/octet/remediation-plan.preserves
cargo test
cargo clippy --all-targets -- -D warnings
CAIRN_ROOT="${CAIRN_ROOT:-../cairn}"
nix run "path:$CAIRN_ROOT#cairn" -- validate --root . \
  --policy "$CAIRN_ROOT/cairn-policy/generated/cairn-policy.json" \
  --strict
```

The strict gate is fail-closed: `warning-only` denies even when `cargo-octet` exits `0`, and `command.txt`, `status.json`, `summary.txt`, structured finding keys, object corpus receipts, and fingerprint evidence are bound by canonical refs in the Octet receipt. The dev shell provides `cargo-octet` from the catalogued `octet-toolchain` flake input (`OnixResearch/octet` `3f7abb641ee604f54629da415bc7686f89662065`, toolchain `nightly-2026-03-21`); an ambient `cargo-octet` elsewhere on `PATH` is not Octet evidence. The [nominal adoption record](../docs/nominal-authority-references.md#result-and-adoption-status) separates the approved focused acceptance from retained whole-core warning debt. The frozen verifier pilot independently retains its original Octet cohort through `verified-node-replication-octet`; see its [exact inputs](../docs/verified-node-replication-pilot.md#exact-inputs). The quarantine commands below are historical burn-down tooling: their example expiry dates have passed, and no baseline or review receipt admits current findings.

The [2026-10-15 local source checkpoint](../docs/octet-burndown-handoff.md#current-source-checkpoint)
records clean configured workspace/library producers and a passing unchanged
`strict-ci` gate. It is unintegrated source evidence, not acceptance of every
active Cairn change.

```sh
cargo run -- test octet baseline write \
  --artifacts target/octet \
  --out target/octet/warning-baseline.preserves \
  --created-at 2026-05-31T00:00:00Z \
  --expires-at 2026-06-30T00:00:00Z
cargo run -- test octet review write \
  --out target/octet/critical-review.preserves \
  --profile quarantine-ci \
  --expires-at 2026-06-30T00:00:00Z \
  --finding-key b3:...
cargo run -- test octet baseline check \
  --artifacts target/octet \
  --baseline target/octet/warning-baseline.preserves \
  --profile quarantine-ci \
  --as-of 2026-05-31T00:00:00Z \
  --review target/octet/critical-review.preserves \
  --receipt-out target/octet/baseline-receipt.preserves
```

## Deterministic harness and replay

```sh
cargo run -- test run examples/two-actor.preserves --report-out target/molten-reports/two-actor.report.preserves
cargo run -- test report validate target/molten-reports/two-actor.report.preserves
cargo run -- test replay target/molten-reports/two-actor.report.preserves
cargo run -- test report show target/molten-reports/two-actor.report.preserves
cargo run -- test gate check target/molten-reports/two-actor.report.preserves --receipt-out target/molten-reports/two-actor.gate-receipt.preserves
cargo run -- test report show target/molten-reports/two-actor.gate-receipt.preserves
cargo run -- test repro export target/molten-reports/two-actor.report.preserves --out target/molten-repro/two-actor
cargo run -- test repro verify target/molten-repro/two-actor/refs.preserves --receipt-out target/molten-repro/two-actor/verify-receipt.preserves
cargo run -- test repro unpack target/molten-repro/two-actor/refs.preserves --out target/molten-repro/two-actor-unpacked
cargo run -- test gate check target/molten-repro/two-actor/refs.preserves --receipt-out target/molten-repro/two-actor/gate-receipt.preserves
cargo run -- test run examples/executor-conformance.preserves --report-out target/molten-reports/executor-conformance.report.preserves
```

This runs a fresh local native two-actor suite through the Preserves harness rail and the in-process Aspen runtime kernel, writes a canonical Preserves report with actor-registry, effect-log, policy-gate, capability-gate, budget-gate, admission-decision, executor hostcall boundary, and budget evidence, replays by injecting recorded clock/random effect responses to the same report/state refs, exports a sealed repro bundle with an embedded report gate receipt, and checks the report or sealed repro bundle as admissible pass evidence with `molten test gate check`. Harness steps now carry canonical Preserves values for message bodies, assertions, retractions, and exact-value observe patterns; string payloads are just the compatibility-friendly shorthand. Suites may include an optional static `<policy-v1 "molten.harness.policy.v1" [...]>` fixture with deny rules such as `<deny "producer" "assert" #f "service.ready" "producer cannot assert readiness">`, and must include explicit `<budget-v1 "molten.harness.budget.v1" ...>`, `<actor-registry-v1 "molten.harness.actor-registry.v1" [...]>`, and `<capabilities-v1 "molten.harness.capabilities.v1" [...]>` fixtures; actor registries bind ids to executor kinds without inferred actors or fallback execution, reviewed Steel hostcall actors may include explicit `<steel-executor-v1 ...>` source/callable/allowed-hostcall fixtures and execute in a reviewed Steel VM with canonical execution receipts, reviewed Wasm hostcall actors may include explicit `<wasm-executor-v1 ...>` module/WIT/allowed-hostcall fixtures validated with `wasmparser` and executed through a no-WASI Wasmtime hostcall shim with optional `molten.wasm.abi.v1` Preserves byte input/output, adapter/remote-proxy kinds require explicit executable preflight fixtures and verified transcript profiles before deterministic gates, capability grants such as `<grant "producer" "send" "consumer" #f>` deny by default when no matching grant exists, and omitted budget/actor/capability fixtures cannot execute or satisfy pass-evidence gates. Denied turns roll back and denied effects do not issue ambient effect requests. Before runtime turns or effects, the harness canonicalizes the policy, capability, and budget snapshots and emits `<policy-gate-v1 "molten.harness.policy-gate.v1" ...>`, `<capability-gate-v1 "molten.harness.capability-gate.v1" ...>`, and `<budget-gate-v1 "molten.harness.budget-gate.v1" ...>` evidence; policy gates now derive deterministic Nickel static source/export refs, validate a Basalt Nickel contract envelope/preflight receipt, and keep unreviewed Steel/dynamic predicate records fail-closed; capability gates validate a Basalt authority contract/preflight receipt, bind explicit empty local UCAN proofsets, and bind every grant ref used by admission authority evidence; budget gates derive deterministic Nickel resource-policy source/export refs and validate a Basalt resource preflight receipt. Report validation fails closed unless the embedded suite has explicit budget/actor/capability fixtures, report budget/actor-registry and policy/capability/budget gate evidence match the embedded suite, every declared actor has matching `<executor-preflight-v1 "molten.runtime.executor-preflight.v1" ...>` evidence with bound executor conformance suite refs, reviewed Steel actor fixtures carry `<steel-review-receipt-v1 ...>` source/callable/allowed-hostcall bindings and admitted Steel steps carry `<steel-execution-receipt-v1 ...>` VM/input/output evidence, reviewed Wasm actor fixtures carry `<wasm-inspection-receipt-v1 ...>` module/import/WIT/allowed-hostcall bindings and admitted Wasm steps carry `<wasm-execution-receipt-v1 ...>` fuel/memory/hostcall/ABI execution evidence, every observation has exactly one matching admission decision with authority evidence, the decision recomputes from embedded capabilities plus policy, actor activity is wrapped in matching `<actor-input-v1 ...>`, `<hostcall-request-v1 ...>`, `<hostcall-decision-v1 ...>`, and `<actor-output-v1 ...>` envelopes, denied turns contain only rollback evidence, and denied effects contain no effect records. Successful gate decisions emit canonical `<gate-receipt-v1 "molten.harness.gate-receipt.v1" ...>` Preserves artifacts containing artifact refs, validation/replay results, explicit-budget/no-default-resource-policy/actor-registry/no-inferred-actors/executor-boundary/executor-preflight/executor-conformance/Steel-review/Wasm-inspection/Wasm-execution/executor-hostcall-boundary/Nickel-policy-source/Nickel-export/Basalt-preflight/Basalt-authority/UCAN-proofset/Nickel-resource-policy/Basalt-resource/capability/hostcall/admission check evidence; without `--receipt-out`, the receipt is emitted on stdout. `molten test repro export --profile deny-sensitive` (the default) writes a sealed `<harness-repro-bundle-v1 ...>` refs artifact plus `gate-receipt.preserves`; the sealed bundle includes redaction policy/gate evidence and refuses sensitive Preserves marker records such as `<secret ...>`, `<confidential ...>`, `<credential ...>`, `<private ...>`, and unvalidated `<encrypted-ref ...>`. `--profile redacted-diagnostic` emits deterministic redaction markers plus canonical redaction transform manifest/receipt files and remains diagnostic-only: `repro verify` and `test gate check` fail closed unless a future policy explicitly marks the transform gate-preserving. `--profile encrypted-private` emits validated `<encrypted-ref-v1 ...>` placeholders and a private-bundle profile; unpacking requires passing reveal receipts via `--reveal-receipt` that bind the exact encrypted-ref ids in the bundle, while missing, stale, malformed, or unauthorized reveal evidence fails closed. `molten test repro verify` emits canonical `<repro-verify-receipt-v1 ...>` receipts for sealed pass bundles, `molten test repro unpack` materializes verified report/suite/receipt contents or diagnostic redacted evidence, and bundle gate checks recompute redaction evidence and embedded report receipts before emitting a new `repro-bundle` gate receipt, while failure repro bundles remain diagnostic-only.

SAM service supervision CLI slice:

```sh
cargo run -- service run-supervision-fixture --out target/molten-service-supervision
cargo run -- service show-supervision target/molten-service-supervision/report.preserves
cargo run -- service replay-supervision target/molten-service-supervision/report.preserves
cargo run -- service gate-supervision target/molten-service-supervision/report.preserves \
  --receipt-out target/molten-service-supervision/gate-receipt.preserves
```

These commands exercise demand-driven service failure supervision over canonical SAM service records. The supervision report carries failure markers, failed/final statuses, lifecycle receipts, deterministic monitor notifications, restart decisions, scheduled restart demand, cleanup receipts, owned-state retractions, and retention inputs as Preserves evidence. `gate-supervision` emits canonical `<service-supervision-gate-receipt-v1 "molten.service.supervision-gate-receipt.v1" ...>` receipts that replay the report, bind status/lifecycle/restart/monitor/cleanup evidence, and stay operational evidence only: they do not grant service authority, resource rights, provenance, or transport trust.

Trellis protocol session CLI slice:

```sh
cargo run -- test protocol run-request-response --out target/molten-protocol
cargo run -- test protocol gate-lifecycle target/molten-protocol \
  --receipt-out target/molten-protocol/gate-receipt.preserves
cargo run -- test protocol show target/molten-protocol/gate-receipt.preserves
```

`run-request-response` installs a finite Trellis-backed protocol manifest, writes endpoint/session/message/operation evidence, and advances projected client/server states through a canonical request/response exchange. `gate-lifecycle` emits `<protocol-session-gate-receipt-v1 "molten.protocol.session-gate-receipt.v1" ...>` after replaying the install receipt and operation receipts against the state/message evidence and checking terminal session state. The gate receipt is replay evidence only; it does not grant protocol authority, resource rights, provenance, policy admission, or transport trust.

Remote SAM/Iroh dataspace CLI slice:

```sh
cargo run -- test remote envelope build \
  --from-peer peer:a --from-actor producer --to-peer peer:b \
  --topic services --operation assert \
  --payload examples/remote-service-ready.preserves \
  --out target/molten-remote/envelope.preserves
cargo run -- test remote publish-local \
  --transport-root target/molten-remote/transport \
  --envelope target/molten-remote/envelope.preserves \
  --node peer:a \
  --receipt-out target/molten-remote/publish.preserves
cargo run -- test remote deliver-local \
  --transport-root target/molten-remote/transport \
  --topic services --envelope-ref blake3:... --receiver-peer peer:b \
  --out target/molten-remote/delivered.preserves \
  --receipt-out target/molten-remote/deliver.preserves
cargo run -- test remote run-two-peer \
  --transport-root target/molten-remote/transport \
  --out target/molten-remote/two-peer
cargo run -- test remote gate \
  --delivery-log target/molten-remote/two-peer/delivery-log.preserves \
  --admission-receipt target/molten-remote/two-peer/admission-receipt.preserves \
  --turn-context-ref $(cat target/molten-remote/two-peer/turn-context-ref.preserves | tr -d '\"') \
  --receipt-out target/molten-remote/two-peer/gate-receipt-2.preserves
```

These commands exercise canonical remote dataspace envelopes over deterministic `iroh-local-gossip`, record delivery logs for replay, and emit remote dataspace gate receipts. Live `iroh-gossip` uses the same library boundary, but unrecorded live timing is not deterministic pass evidence.

Failure artifacts are canonical Preserves evidence too. Failures emit `<harness-failure-v1 "molten.harness.failure.v1" ...>` on stdout by default, or write it to `--report-out`/`--failure-out` when supplied, and still exit non-zero:

```sh
cargo run -- test run bad-suite.preserves --report-out target/molten-reports/bad.failure.preserves
cargo run -- test report show target/molten-reports/bad.failure.preserves
cargo run -- test replay target/molten-reports/tampered.report.preserves --failure-out target/molten-reports/replay.failure.preserves
cargo run -- test report validate target/molten-reports/tampered.report.preserves --failure-out target/molten-reports/validate.failure.preserves
cargo run -- test repro export target/molten-reports/tampered.report.preserves --out target/molten-repro/tampered --failure-out target/molten-reports/export.failure.preserves
cargo run -- test gate check target/molten-reports/bad.failure.preserves --failure-out target/molten-reports/gate.failure.preserves
```

## Nix

```sh
nix build
nix flake check
nix build .#checks.x86_64-linux.verified-node-replication-pilot --no-link -L
```

## Release builds

r[impl molten.project.release_overflow_checks] Molten-owned crates (`molten`, `molten-core`, `molten-node-host`, `molten-release-policy`) trap integer overflow in release builds. `Cargo.toml` sets `overflow-checks = true` for each in `[profile.release.package.*]`, and `flake.nix` passes `-C overflow-checks=on` to the same crates through `workspaceCrateOverrides`, because nixpkgs `buildRustCrate` ignores Cargo profiles. The `release-overflow-checks` flake check fails if a member crate's derivation lacks the option. Third-party crates keep their own release defaults in both builds.

A trap fails closed instead of wrapping an index, counter or budget. Arithmetic that can legitimately reach a bound still uses checked or saturating operations with an explicit outcome, and code reachable from an `extern "C"` entry point, such as the SpaceWasm allocator, never relies on a trap. Overflow checks do not police lossy `as` conversions. The cost was not measured; reopen the choice with paired representative measurements if a profile shows a regression.
