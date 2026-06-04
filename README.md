# molten

Rust project scaffolded with a Nix flake.

Includes Rust dependencies for Steel Scheme (`steel-core`), iroh (`iroh`, `iroh-blobs`, `iroh-docs`, `iroh-gossip`), Syndicate (`syndicate`), Preserves (`preserves`, `preserves-schema`), Nickel (`nickel-lang`), Wasmtime and WASI/component tooling (`wasmtime`, `wasmtime-wasi`, `wit-bindgen`, `wasmparser`), Redb (`redb`), Blake3 (`blake3`), Snafu (`snafu`), Serde (`serde`), Tracing (`tracing`), Clap (`clap`), and OnixResearch git dependencies for Basalt (`basalt`), Cairn (`cairn-core` as `cairn`), Octet (`valence-core` as `octet`), and Trellis (`verified-logic` as `trellis`). Dev dependencies include Hegel (`hegeltest`, imported as `hegel`) for property-based testing. The Nix dev shell also exposes the `steel`, `nickel`, and `wasmtime` CLIs.

## Architecture direction

See [`docs/architecture.md`](docs/architecture.md) for the fuller architecture.

Molten is a policy-gated distributed runtime built around a canonical Preserves envelope spine:

- Deterministic playback is a central law: the same artifacts, dependency closure, initial state, policy/schema refs, handler profile, and seed or recorded effect log must reproduce the same canonical traces, receipts, outputs, and final state hash.
- Preserves + Blake3 define stable communication, storage, policy, and evidence boundaries.
- Synit/SAM-inspired dataspaces provide assertions, retractions, `Observe` patterns, service dependency assertions, and turn tracing.
- Spritely Goblins-inspired vats are optional actor internals for near/far object references, transactional actormaps, promises/vows, revocable proxies, safe object serialization, and authority-graph debugging.
- Trellis choreographies define finite multi-party protocol shape and project to dataspace-backed local endpoints.
- Trellis Raft primitives define strongly consistent replicated control-plane state, not normal actor traffic.
- Basalt/UCAN, Nickel contracts, reviewed Steel predicates, Trellis predicates, Cairn receipts, and Octet/Valence evidence gate side effects.
- Iroh bridges envelopes, blobs, and docs across peers; Wasmtime actors run behind deny-by-default hostcalls; Redb stores local durable metadata and indexes.

Current Cairn roadmap changes live under `cairn/changes/`:

- `runtime-spine`
- `synit-sam-runtime`
- `goblins-vat-runtime`
- `trellis-choreography`
- `trellis-raft-consensus`
- `trellis-runtime-predicates`
- `unison-artifact-registry`
- `unison-effect-handlers`
- `unison-remote-artifact-sync`
- `unison-typed-storage`
- `unison-upgrade-sessions`
- `unison-schema-identity`
- `unison-evaluation-cache`
- `unison-executable-transcripts`
- `unison-structured-rewrite`
- `unison-artifact-catalog-mcp`
- `unison-distributed-job-dag`
- `deterministic-test-playback`
- `first-class-testing-harness`
- `admission-evidence-validation`
- `policy-boundary-preflight`
- `nickel-basalt-policy-preflight`
- `capability-context-admission`
- `basalt-ucan-capability-preflight`
- `mandatory-capability-fixtures`
- `mandatory-actor-registry-fixtures`
- `mandatory-budget-fixtures`
- `nickel-basalt-budget-preflight`
- `sealed-repro-bundles`
- `sealed-repro-verify-unpack`
- `sealed-repro-redaction-preflight`
- `redacted-repro-export-profiles`
- `signed-evidence-receipts`
- `chain-hashed-evidence-ledger`
- `local-evidence-ledger-store`
- `executor-hostcall-boundary`
- `iroh-sealed-repro-exchange`
- `haskell-runtime-patterns`
- `octet-enforcement-gates`
- `authority-identity-revocation`
- `resource-governance-backpressure`
- `delivery-idempotency-replay`
- `failure-supervision-lifecycle`
- `retention-gc-pinning`
- `secrets-redaction-confidentiality`
- `supply-chain-provenance-builds`
- `peer-bootstrap-negotiation`
- `content-addressed-chunk-store`
- `persistent-node-identity`
- `operator-receipts-dogfood`
- `plugin-host-abi`
- `federated-pull-sync`
- `blob-ref-job-submission`
- `coordination-primitives`
- `iroh-sam-dataspace`
- `remote-dataspace-harness-cli`
- `molten-node-runtime-daemon`
- `node-control-socket-runtime`
- `node-control-operation-dispatch`
- `node-control-daemon-loop`
- `node-control-provenance-gates`
- `node-control-iroh-ingress`
- `node-control-supervised-runner`
- `node-control-live-iroh-transport`
- `node-control-live-serve-listener`
- `node-control-authority-delegation`
- `node-control-live-peer-tickets`
- `node-control-supervisor-policy`
- `sam-service-supervision-runtime`
- `sam-service-records-ledger`
- `sam-service-demand-runtime`
- `sam-service-supervision-cleanup`
- `trellis-protocol-session-runtime`
- `raft-control-plane-registry`
- `job-dag-iroh-worker-execution`
- `dataspace-delivery-idempotency`
- `secrets-redaction-encrypted-refs`
- `plugin-host-lifecycle-runtime`
- `coordination-services-control-plane`
- `operator-dogfood-node-workflow`

## Node runtime daemon

The durable local node boundary is exposed at top level under `molten node`:

```sh
molten node init --state-root target/node --node-id node:local
molten node run --state-root target/node
molten node control-request --operation status --out target/node.status-request.preserves \
  --authority blake3:authority --policy blake3:policy --resource blake3:resource
molten node provenance-fixture --artifact-ref blake3:payload --out target/node.payload-provenance.preserves
molten node authority-grant-fixture --state-root target/node --peer peer:operator --node node:local \
  --operation status --policy blake3:policy --out target/node.operator-grant.preserves
molten node live-ticket-export --state-root target/node --policy blake3:policy \
  --out target/node.live-ticket.preserves
molten node live-peer-admit --state-root target/node --peer peer:operator --policy blake3:policy \
  --receipt-out target/node.operator-peer-admission.preserves target/node.live-ticket.preserves
molten node supervisor-policy-fixture --state-root target/node --max-restarts 1 \
  --restart-window-ticks 8 --heartbeat-timeout-ticks 4 --shutdown-drain-ticks 2 \
  --allow-stale-lock-recovery --policy blake3:policy \
  --out target/node.supervisor-policy.preserves
molten node control-submit --state-root target/node target/node.status-request.preserves \
  --receipt-out target/node.status-queue.preserves
molten node control-ingress-build target/node.status-request.preserves \
  --from-peer peer:operator --to-node node:local --peer-bootstrap blake3:bootstrap \
  --authority blake3:authority --policy blake3:policy --resource blake3:resource \
  --out target/node.status-ingress.preserves
molten node control-ingress-publish --state-root target/node target/node.status-ingress.preserves \
  --receipt-out target/node.status-ingress-publish.preserves
molten node control-ingress-deliver --state-root target/node blake3:ingress-envelope \
  --receipt-out target/node.status-ingress-deliver.preserves
molten node control-ingress-live-loopback --state-root target/node target/node.status-request.preserves \
  --from-peer peer:operator --to-node node:local --peer-bootstrap blake3:peer-admission \
  --authority blake3:operator-grant --policy blake3:policy --resource blake3:resource \
  --receive-receipt-out target/node.status-live-receive.preserves
molten node run-loop --state-root target/node --max-requests 8 \
  --receipt-out target/node.control-loop.preserves
molten node serve --state-root target/node --max-ticks 64 --max-requests-per-tick 8 \
  --supervisor-policy target/node.supervisor-policy.preserves \
  --receipt-out target/node.service-run.preserves
molten node serve --state-root target/node --live-iroh --live-max-events 16 \
  --service-receipt-out target/node.service-run.preserves \
  --live-ticket-out target/node.bound-live-ticket.preserves \
  --receipt-out target/node.live-listener.preserves
molten node control-dispatch --state-root target/node --receipt-out target/node.status-dispatch.preserves
molten node status --state-root target/node --health-out target/node.health.preserves
molten node stop --state-root target/node --shutdown-out target/node.shutdown.preserves
molten node show target/node/startup-receipt.preserves
```

`init` writes canonical `node-config-v1` and node identity receipts under the explicit state root. `run` validates the source gate, starts required adapters in deterministic dependency order, writes an active startup-bound node lock, and emits `node-startup-receipt-v1` plus adapter receipts. `control-submit` persists canonical requests in the explicit state-root inbox and writes queue receipts; `control-ingress-build`/`publish`/`deliver` provide deterministic local-Iroh ingress that validates peer bootstrap, authority, policy, resource, and delivery-idempotency evidence before enqueue; `control-ingress-live-build` and `control-ingress-live-loopback` exercise the real `iroh-gossip` transport with canonical envelope bytes and live transport receipts while still feeding the durable ingress path, where live peer bootstrap refs must resolve to admitted `node-control-live-peer-admission-v1` ticket evidence and live authority refs must resolve to admitted `node-control-authority-grant-v1` ledger artifacts. `live-ticket-export` and `live-peer-admit` provide deterministic peer bootstrap tickets/admissions; `authority-grant-fixture` emits/imports deterministic grants for local live-ingress workflows; transport identity and neighbor evidence never count as bootstrap or authority. `supervisor-policy-fixture` emits/imports canonical `node-control-supervisor-policy-v1` bounds for restart attempts, restart windows, heartbeat timeouts, shutdown drain, and stale-lock recovery. `serve` acquires a separate service lock, emits service heartbeat/run receipts, scans local-Iroh ingress in deterministic order, and drains via the existing bounded control loop until the tick bound or shutdown stop; optional `--supervisor-policy` imports the policy, emits supervisor receipts for restart admission/denial, stale-lock recovery, duplicate-runner denial, and shutdown drain bounds, and fails closed when a stale service lock lacks recovery policy or restart bounds are exceeded; `serve --live-iroh` first runs a bounded live Iroh listener, records neighbor/session observations and live transport receipts, then drains through the same supervised control loop; `control-dispatch` requires the active lock, emits dispatch receipts, routes `status`/`shutdown`, and dispatches side-effecting `install`/`run`/`gate` operations through ledger-resolved payloads before importing operation subreceipts into the node ledger. `control-request` accepts explicit `--evidence` refs, and `provenance-fixture` emits a synthetic reviewed `provenance-record-v1` for local tests. `run-loop` drains the inbox in deterministic path order up to `--max-requests`, emits heartbeat and loop receipts, returns prior receipts for duplicate request refs, and stops after a passing shutdown dispatch removes the active lock. `install` writes node-control artifacts into the node registry only after reviewed/reproducible/policy-trusted provenance for the payload ref passes; `run` executes a node-local job execution request only after admitted provenance for the job ref passes; and `gate` validates strict Octet source-gate evidence for the target subject. Missing peer bootstrap, live ticket admission, authority, policy, resource, provenance, operation-required target/payload, duplicate service lock, live delegation grant, or ledger evidence fails closed before enqueue or operation side effects. `status` and `stop` are convenience paths over local Preserves control requests whose rendered text is non-normative; the canonical evidence is the emitted service/ingress/queue/control/health/shutdown/provenance/authority receipts imported into the node ledger.

## Development

```sh
nix develop
cargo nextest run
cargo test # fallback
```

Nextest profiles:

```sh
cargo nextest run --profile ci
cargo nextest run --profile deterministic
cargo nextest run --profile exploratory
```

Nix exposes the CI command and runs nextest through flake checks:

```sh
nix run .#nextest-ci
nix build .#checks.x86_64-linux.nextest
nix build .#checks.x86_64-linux.nextest-config
```

For the current private OnixResearch git dependencies, the flake locks local Cargo checkout sources as `*-src` path inputs and unit2nix serves those checkouts to Cargo's git cache. This keeps the Nix builder from needing SSH access to GitHub. Latest local Nix nextest evidence: `nix build .#checks.x86_64-linux.nextest --no-link --print-out-paths --option eval-cache false --option substituters https://cache.nixos.org/ --option builders "" --option auto-optimise-store false --option min-free 0 --option max-free 0` -> `/nix/store/8fcxgyj17dkigp5idpvnzb5dv78nd4nz-molten-nextest`.

Strict Octet source-gate sequence:

```sh
cargo octet check --artifact-dir target/octet
cargo octet check -p molten --artifact-dir target/octet-lib -- --lib
cargo octet object corpus receipt \
  --output target/octet/object-corpus-receipt.json \
  src/artifacts.rs src/catalog.rs src/catalog_mcp.rs \
  src/coordination.rs src/delivery_idempotency.rs src/job_dag.rs \
  src/main.rs src/node_daemon.rs src/node_runtime.rs src/octet_gate.rs \
  src/operator_dogfood.rs \
  src/plugin_host.rs src/provenance.rs src/raft_control_plane.rs src/remote_dataspace.rs \
  src/secrets.rs src/transcripts.rs
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
cargo run --manifest-path /home/brittonr/.cargo/git/checkouts/cairn-d7a4d31a0615cac1/3b4c280/Cargo.toml \
  -p cairn-cli --bin cairn -- validate --root . \
  --policy /home/brittonr/.cargo/git/checkouts/cairn-d7a4d31a0615cac1/3b4c280/cairn-policy/generated/cairn-policy.json \
  --strict
```

The strict gate is fail-closed: `warning-only` denies even when `cargo-octet` exits `0`, and `command.txt`, `status.json`, `summary.txt`, structured finding keys, object corpus receipts, and fingerprint evidence are bound by canonical refs in the Octet receipt. Current remediation snapshot: workspace and lib-only Octet are `clean` with 0 findings, 0 warnings, and 0 errors; focused object corpus has 1338 objects (`b3:0fb63563911d4ad22d5476ed31337453c44d506f99b9ceeadeee962ac945c45a`); latest artifact import receipt is `blake3:4616b7ec0499aa384c7da2634ebfdad8e3e29c0e4be5b756ff45936a97be2eab`, latest strict pass receipt is `blake3:703d7c66589dfe020db841a78f98e294251d14ae850a1e784695d0811a5889cf`, and latest remediation plan receipt is `blake3:b0e644bfc5c812950666eaefecc8eb38f7bb0bc8a65a2c648e1249184259cd7f`. Caveat: this is configuration-clean with the broad high-noise lint families explicitly disabled in `dylint.toml`; source-remediated zero for those disabled families remains separate follow-up work. During warning burn-down only, use the explicit quarantine flow:

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

First deterministic harness/runtime slice:

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

This runs a fresh local native two-actor suite through the Preserves harness rail and the in-process Molten runtime kernel, writes a canonical Preserves report with actor-registry, effect-log, policy-gate, capability-gate, budget-gate, admission-decision, executor hostcall boundary, and budget evidence, replays by injecting recorded clock/random effect responses to the same report/state refs, exports a sealed repro bundle with an embedded report gate receipt, and checks the report or sealed repro bundle as admissible pass evidence with `molten test gate check`. Harness steps now carry canonical Preserves values for message bodies, assertions, retractions, and exact-value observe patterns; string payloads are just the compatibility-friendly shorthand. Suites may include an optional static `<policy-v1 "molten.harness.policy.v1" [...]>` fixture with deny rules such as `<deny "producer" "assert" #f "service.ready" "producer cannot assert readiness">`, and must include explicit `<budget-v1 "molten.harness.budget.v1" ...>`, `<actor-registry-v1 "molten.harness.actor-registry.v1" [...]>`, and `<capabilities-v1 "molten.harness.capabilities.v1" [...]>` fixtures; actor registries bind ids to executor kinds without inferred actors or fallback execution, reviewed Steel hostcall actors may include explicit `<steel-executor-v1 ...>` source/callable/allowed-hostcall fixtures and execute in a reviewed Steel VM with canonical execution receipts, reviewed Wasm hostcall actors may include explicit `<wasm-executor-v1 ...>` module/WIT/allowed-hostcall fixtures validated with `wasmparser` and executed through a no-WASI Wasmtime hostcall shim with optional `molten.wasm.abi.v1` Preserves byte input/output, adapter/remote-proxy kinds require explicit executable preflight fixtures and verified transcript profiles before deterministic gates, capability grants such as `<grant "producer" "send" "consumer" #f>` deny by default when no matching grant exists, and omitted budget/actor/capability fixtures cannot execute or satisfy pass-evidence gates. Denied turns roll back and denied effects do not issue ambient effect requests. Before runtime turns or effects, the harness canonicalizes the policy, capability, and budget snapshots and emits `<policy-gate-v1 "molten.harness.policy-gate.v1" ...>`, `<capability-gate-v1 "molten.harness.capability-gate.v1" ...>`, and `<budget-gate-v1 "molten.harness.budget-gate.v1" ...>` evidence; policy gates now derive deterministic Nickel static source/export refs, validate a Basalt Nickel contract envelope/preflight receipt, and keep unreviewed Steel/dynamic predicate records fail-closed; capability gates validate a Basalt authority contract/preflight receipt, bind explicit empty local UCAN proofsets, and bind every grant ref used by admission authority evidence; budget gates derive deterministic Nickel resource-policy source/export refs and validate a Basalt resource preflight receipt. Report validation fails closed unless the embedded suite has explicit budget/actor/capability fixtures, report budget/actor-registry and policy/capability/budget gate evidence match the embedded suite, every declared actor has matching `<executor-preflight-v1 "molten.runtime.executor-preflight.v1" ...>` evidence with bound executor conformance suite refs, reviewed Steel actor fixtures carry `<steel-review-receipt-v1 ...>` source/callable/allowed-hostcall bindings and admitted Steel steps carry `<steel-execution-receipt-v1 ...>` VM/input/output evidence, reviewed Wasm actor fixtures carry `<wasm-inspection-receipt-v1 ...>` module/import/WIT/allowed-hostcall bindings and admitted Wasm steps carry `<wasm-execution-receipt-v1 ...>` fuel/memory/hostcall/ABI execution evidence, every observation has exactly one matching admission decision with authority evidence, the decision recomputes from embedded capabilities plus policy, actor activity is wrapped in matching `<actor-input-v1 ...>`, `<hostcall-request-v1 ...>`, `<hostcall-decision-v1 ...>`, and `<actor-output-v1 ...>` envelopes, denied turns contain only rollback evidence, and denied effects contain no effect records. Successful gate decisions emit canonical `<gate-receipt-v1 "molten.harness.gate-receipt.v1" ...>` Preserves artifacts containing artifact refs, validation/replay results, explicit-budget/no-default-resource-policy/actor-registry/no-inferred-actors/executor-boundary/executor-preflight/executor-conformance/Steel-review/Wasm-inspection/Wasm-execution/executor-hostcall-boundary/Nickel-policy-source/Nickel-export/Basalt-preflight/Basalt-authority/UCAN-proofset/Nickel-resource-policy/Basalt-resource/capability/hostcall/admission check evidence; without `--receipt-out`, the receipt is emitted on stdout. `molten test repro export` now writes a sealed `<harness-repro-bundle-v1 ...>` refs artifact plus `gate-receipt.preserves`; the sealed bundle includes redaction policy/gate evidence and refuses sensitive Preserves marker records such as `<secret ...>`, `<confidential ...>`, `<credential ...>`, `<private ...>`, and unvalidated `<encrypted-ref ...>`. `molten test repro verify` emits canonical `<repro-verify-receipt-v1 ...>` receipts for sealed bundles, `molten test repro unpack` materializes verified report/suite/receipt contents, and bundle gate checks recompute both redaction evidence and the embedded report receipt before emitting a new `repro-bundle` gate receipt, while failure repro bundles remain diagnostic-only.

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
```
