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
- `supply-chain-provenance-ux`
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
- `node-control-live-send-ux`
- `node-control-live-import-ux`
- `node-control-live-send-diagnostics`
- `node-control-live-workflow-bundle-import-export`
- `node-control-live-workflow-bundle-verify`
- `node-control-live-workflow-bundle-gate`
- `node-control-live-workflow-bundle-apply`
- `node-control-live-workflow-bundle-reconcile`
- `node-control-live-workflow-bundle-ack`
- `node-control-live-trellis-workflow`
- `sam-service-supervision-runtime`
- `sam-service-records-ledger`
- `sam-service-demand-runtime`
- `sam-service-supervision-cleanup`
- `trellis-protocol-session-runtime`
- `raft-control-plane-registry`
- `job-dag-iroh-worker-execution`
- `job-dag-iroh-worker-cli-ux`
- `job-worker-coordination-scheduling-ux`
- `dataspace-delivery-idempotency`
- `secrets-redaction-encrypted-refs`
- `plugin-host-lifecycle-runtime`
- `coordination-services-control-plane`
- `coordination-control-plane-ux`
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
molten node live-ticket-import --state-root target/sender-node target/node.live-ticket.preserves \
  --peer-admission target/node.operator-peer-admission.preserves \
  --expected-node node:local --expected-topic node-control --expected-peer peer:operator \
  --receipt-out target/sender-node.live-ticket-import.preserves
molten node authority-grant-import --state-root target/sender-node target/node.operator-grant.preserves \
  --peer peer:operator --node node:local --operation status \
  --receipt-out target/sender-node.authority-grant-import.preserves
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
molten node control-ingress-live-send --state-root target/sender-node \
  target/node.status-request.preserves target/node.bound-live-ticket.preserves \
  --from-peer peer:operator --operation-id blake3:expected-operation \
  --expected-node node:local --expected-topic node-control --max-attempts 2 \
  --peer-bootstrap blake3:peer-admission \
  --authority blake3:operator-grant --policy blake3:policy --resource blake3:resource \
  --transport-receipt-out target/node.status-live-send-transport.preserves \
  --retry-receipts-dir target/node.live-send-retries \
  --duplicate-receipt-out target/node.status-live-duplicate.preserves \
  --receipt-out target/node.status-live-send.preserves
molten node run-loop --state-root target/node --max-requests 8 \
  --receipt-out target/node.control-loop.preserves
molten node serve --state-root target/node --max-ticks 64 --max-requests-per-tick 8 \
  --supervisor-policy target/node.supervisor-policy.preserves \
  --receipt-out target/node.service-run.preserves
molten node serve --state-root target/node --live-iroh --live-max-events 16 \
  --service-receipt-out target/node.service-run.preserves \
  --live-ticket-out target/node.bound-live-ticket.preserves \
  --receipt-out target/node.live-listener.preserves
molten node live-workflow-bundle-export \
  --ticket target/node.bound-live-ticket.preserves \
  --peer-admission target/node.operator-peer-admission.preserves \
  --authority-grant target/node.operator-grant.preserves \
  --receipt target/node.status-live-send.preserves \
  --out target/node.live-workflow-bundle.preserves \
  --receipt-out target/node.live-workflow-bundle-export.preserves
molten node live-workflow-bundle-verify target/node.live-workflow-bundle.preserves \
  --expected-node node:local --expected-topic node-control --expected-peer peer:operator \
  --operation status --receipt-out target/node.live-workflow-bundle-verify.preserves
molten node live-workflow-bundle-gate target/node.live-workflow-bundle.preserves \
  --verify-receipt target/node.live-workflow-bundle-verify.preserves --require-verify-receipt \
  --expected-node node:local --expected-topic node-control --expected-peer peer:operator \
  --operation status --receipt-out target/node.live-workflow-bundle-gate.preserves
molten node live-workflow-bundle-apply --state-root target/sender-node \
  target/node.live-workflow-bundle.preserves \
  --gate-receipt target/node.live-workflow-bundle-gate.preserves --require-gate-receipt \
  --expected-node node:local --expected-topic node-control --expected-peer peer:operator \
  --operation status --receipt-out target/sender-node.live-workflow-bundle-apply.preserves
molten node live-workflow-bundle-reconcile \
  target/sender-node.live-workflow-bundle-apply.preserves \
  --ingress-receipt target/node.status-ingress-deliver.preserves \
  --queue-receipt target/node.status-queue.preserves \
  --control-receipt target/node.status-dispatch.preserves \
  --receipt-out target/node.live-workflow-bundle-reconcile.preserves
molten node live-workflow-bundle-ack-export \
  target/sender-node.live-workflow-bundle-apply.preserves \
  --ingress-receipt target/node.status-ingress-deliver.preserves \
  --queue-receipt target/node.status-queue.preserves \
  --control-receipt target/node.status-dispatch.preserves \
  --reconcile-receipt target/node.live-workflow-bundle-reconcile.preserves \
  --out target/node.live-workflow-bundle-ack.preserves \
  --receipt-out target/node.live-workflow-bundle-ack-export.preserves
molten node live-workflow-bundle-ack-import --state-root target/sender-node \
  target/node.live-workflow-bundle-ack.preserves \
  --receipt-out target/sender-node.live-workflow-bundle-ack-import.preserves
molten node live-workflow-bundle-protocol-gate target/node.live-workflow-bundle.preserves \
  --gate-receipt target/node.live-workflow-bundle-gate.preserves \
  --apply-receipt target/sender-node.live-workflow-bundle-apply.preserves \
  --reconcile-receipt target/node.live-workflow-bundle-reconcile.preserves \
  --ack target/node.live-workflow-bundle-ack.preserves \
  --receipt-out target/node.live-workflow-protocol-gate.preserves
molten node live-workflow-bundle-import --state-root target/sender-node \
  target/node.live-workflow-bundle.preserves \
  --expected-node node:local --expected-topic node-control --expected-peer peer:operator \
  --operation status --receipt-out target/sender-node.live-workflow-bundle-import.preserves
molten node live-workflow-bundle --state-root target/node \
  --ticket target/node.bound-live-ticket.preserves \
  --peer-admission target/node.operator-peer-admission.preserves \
  --authority-grant target/node.operator-grant.preserves \
  --send-receipt target/node.status-live-send.preserves \
  --receive-receipt target/node.status-live-receive.preserves \
  --listener-receipt target/node.live-listener.preserves \
  --service-receipt target/node.service-run.preserves \
  --receipt-out target/node.live-workflow.preserves
molten node control-dispatch --state-root target/node --receipt-out target/node.status-dispatch.preserves
molten node status --state-root target/node --health-out target/node.health.preserves
molten node stop --state-root target/node --shutdown-out target/node.shutdown.preserves
molten node show target/node/startup-receipt.preserves
```

`init` writes canonical `node-config-v1` and node identity receipts under the explicit state root. `run` validates the source gate, starts required adapters in deterministic dependency order, writes an active startup-bound node lock, and emits `node-startup-receipt-v1` plus adapter receipts. `control-submit` persists canonical requests in the explicit state-root inbox and writes queue receipts; `control-ingress-build`/`publish`/`deliver` provide deterministic local-Iroh ingress that validates peer bootstrap, authority, policy, resource, and delivery-idempotency evidence before enqueue; `control-ingress-live-build` and `control-ingress-live-loopback` exercise the real `iroh-gossip` transport with canonical envelope bytes and live transport receipts while still feeding the durable ingress path, where live peer bootstrap refs must resolve to admitted `node-control-live-peer-admission-v1` ticket evidence and live authority refs must resolve to admitted `node-control-authority-grant-v1` ledger artifacts. `live-ticket-export` and `live-peer-admit` provide deterministic peer bootstrap tickets/admissions; `live-ticket-import` imports receiver tickets and optional peer-admission receipts into another explicit state root after node/topic/endpoint/peer/freshness validation; `authority-grant-fixture` emits/imports deterministic grants for local live-ingress workflows, and `authority-grant-import` imports moved grants after peer/node/operation/scope/epoch/revocation validation. Import receipts are operational evidence only: they do not replace peer admissions, grants, policy/resource, or provenance refs. Transport identity and neighbor evidence never count as bootstrap or authority. `control-ingress-live-send` uses a bound live ticket from `serve --live-ticket-out` to join the receiver's real `iroh-gossip` topic, publish canonical live ingress bytes, and emit `node-control-live-send-receipt-v1` plus live transport receipts without satisfying authority or provenance by transport alone. It also exposes an `--operation-id` guard over the derived envelope operation ref, `--expected-node`/`--expected-topic`/`--expected-endpoint` ticket guards, sender-state-root preflight for imported peer admissions and authority grants, bounded `--max-attempts`, canonical live-send retry receipts, and duplicate-send receipts that suppress repeat broadcasts when a prior pass send receipt already exists. Missing imports, wrong ticket bindings, unsupported addresses, operation mismatches, and join/publish failures are classified in live-send receipt checks (`receiver-ticket-expected`, `receiver-address-supported`, `operation-id-bound`, `sender-state-root-evidence`, `join-or-publish-succeeded`) with deterministic import guidance. `live-workflow-bundle-export` packages a ticket, peer admission, authority grant, and supporting receipts into a canonical `node-control-live-workflow-bundle-v1` handoff artifact; `live-workflow-bundle-verify` checks the handoff offline and emits a non-authority verify receipt; `live-workflow-bundle-gate` rechecks the bundle and optionally requires a current matching verify receipt before import; `live-workflow-bundle-apply` revalidates the gate, imports ticket/admission/grant members, optionally dry-runs a request preflight, and only sends over live Iroh when `--send` is explicitly provided while emitting a non-authority apply receipt; `live-workflow-bundle-reconcile` checks apply/send evidence against receiver ingress, queue, and optional control receipts so operators can prove enqueue/dispatch outcomes without making reconcile receipts authoritative; `live-workflow-bundle-ack-export` packages reconcile plus receiver evidence into a portable non-authority ack bundle, and `live-workflow-bundle-ack-import` imports that ack into the sender ledger after ref/operation/request guards; `live-workflow-bundle-protocol-gate` projects bundle handoff, apply evidence, and ack evidence through a finite Trellis sender/receiver protocol and emits a `protocol-session-gate-receipt-v1` that also denies on failed or mismatched workflow evidence while remaining non-authority/non-provenance; `live-workflow-bundle-import` validates that handoff and imports the underlying ticket/admission/grant artifacts into the sender state root while keeping bundle receipts non-authority. `live-workflow-bundle` ties ticket, peer admission, authority grant, send, receive/listener, and service-run receipts into a canonical `node-control-live-workflow-receipt-v1` operator runbook receipt. `supervisor-policy-fixture` emits/imports canonical `node-control-supervisor-policy-v1` bounds for restart attempts, restart windows, heartbeat timeouts, shutdown drain, and stale-lock recovery. `serve` acquires a separate service lock, emits service heartbeat/run receipts, scans local-Iroh ingress in deterministic order, and drains via the existing bounded control loop until the tick bound or shutdown stop; optional `--supervisor-policy` imports the policy, emits supervisor receipts for restart admission/denial, stale-lock recovery, duplicate-runner denial, and shutdown drain bounds, and fails closed when a stale service lock lacks recovery policy or restart bounds are exceeded; `serve --live-iroh` first runs a bounded live Iroh listener, records neighbor/session observations and live transport receipts, then drains through the same supervised control loop; `control-dispatch` requires the active lock, emits dispatch receipts, routes `status`/`shutdown`, and dispatches side-effecting `install`/`run`/`gate` operations through ledger-resolved payloads before importing operation subreceipts into the node ledger. `control-request` accepts explicit `--evidence` refs, and `provenance-fixture` emits a synthetic reviewed `provenance-record-v1` for local tests. `run-loop` drains the inbox in deterministic path order up to `--max-requests`, emits heartbeat and loop receipts, returns prior receipts for duplicate request refs, and stops after a passing shutdown dispatch removes the active lock. `install` writes node-control artifacts into the node registry only after reviewed/reproducible/policy-trusted provenance for the payload ref passes; `run` executes a node-local job execution request only after admitted provenance for the job ref passes; and `gate` validates strict Octet source-gate evidence for the target subject. Missing peer bootstrap, live ticket admission, authority, policy, resource, provenance, operation-required target/payload, duplicate service lock, live delegation grant, or ledger evidence fails closed before enqueue or operation side effects. `status` and `stop` are convenience paths over local Preserves control requests whose rendered text is non-normative; the canonical evidence is the emitted service/ingress/queue/control/health/shutdown/provenance/authority receipts imported into the node ledger.

## Delivery idempotency diagnostics

The delivery idempotency UX exposes canonical operation identity and dedup checks without running a full live transport loop:

```sh
molten test delivery scope --scope-profile remote-dataspace-topic \
  --scope-name peer:b:services --retention-ref blake3:policy \
  --out target/delivery.scope.preserves
molten test delivery operation-id --scope-profile remote-dataspace-topic \
  --scope-name peer:b:services --producer peer:a/producer --consumer peer:b \
  --sequence 1 --intent remote-dataspace-assert --payload-ref blake3:payload \
  --policy-ref blake3:policy --out target/delivery.operation.preserves
molten test delivery check --root target/delivery-store \
  --scope-profile remote-dataspace-topic --scope-name peer:b:services \
  --producer peer:a/producer --consumer peer:b --sequence 1 \
  --intent remote-dataspace-assert --payload-ref blake3:payload \
  --policy-ref blake3:policy --evidence-ref blake3:evidence \
  --semantic-result-ref blake3:result \
  --receipt-out target/delivery.first.preserves
```

`first` receipts permit the caller to commit its side effect; exact duplicates emit `duplicate` receipts with side effect `suppress` and a prior receipt ref; conflicts, stale sequences, and gaps deny or retry before side effects. These receipts are replay/dedup evidence only and do not grant transport, authority, provenance, policy, resource, or execution trust.

## Supply-chain provenance diagnostics

The provenance UX exposes canonical trust-state records and evaluation receipts without running a full node-control dispatch:

```sh
molten test provenance fixture --artifact-ref blake3:artifact \
  --out target/provenance.reviewed.preserves
molten test provenance record --artifact-ref blake3:artifact --trust-state reviewed \
  --source-ref blake3:source --dependency-closure-ref blake3:deps \
  --toolchain-ref blake3:toolchain --builder-ref blake3:builder \
  --review-ref blake3:review --test-ref blake3:tests \
  --source-gate-ref blake3:octet --policy-ref blake3:policy \
  --out target/provenance.record.preserves
molten test provenance evaluate --operation install --profile node-control \
  --artifact-ref blake3:artifact --provenance target/provenance.record.preserves \
  --receipt-out target/provenance.receipt.preserves
molten test provenance show target/provenance.receipt.preserves
molten test provenance build-record --expected-artifact-ref blake3:artifact \
  --source-ref blake3:source --dependency-closure-ref blake3:deps \
  --toolchain-ref blake3:toolchain --build-param target=x86_64-linux \
  --builder-ref blake3:builder --nix-derivation-ref blake3:drv \
  --policy-ref blake3:policy --evidence-ref blake3:octet \
  --out target/provenance.build-record.preserves
molten test provenance verify-build target/provenance.build-record.preserves \
  --actual-artifact-ref blake3:artifact \
  --receipt-out target/provenance.build-verify.preserves
molten test provenance record --artifact-ref blake3:artifact \
  --trust-state reproducible-verified --source-ref blake3:source \
  --dependency-closure-ref blake3:deps --toolchain-ref blake3:toolchain \
  --builder-ref blake3:builder --build-record-ref blake3:build-record \
  --out target/provenance.reproducible.preserves
molten test provenance evaluate --operation install --profile node-control \
  --artifact-ref blake3:artifact --provenance target/provenance.reproducible.preserves \
  --build-verification target/provenance.build-verify.preserves \
  --receipt-out target/provenance.reproducible.receipt.preserves
```

Provenance receipts explain whether an artifact's explicit provenance is admitted for a profile; build verification receipts explain whether a reproducible build record's expected artifact matches the actual artifact. A `reproducible-verified` provenance record is denied unless a matching passing build verification receipt references a build record ref bound by that provenance record. Sensitive operations such as policy-artifact, migration-recipe, production-executable, and remote execution admission require stronger `reproducible-verified` or `policy-trusted` provenance. Job `sync-loopback` validates supplied `--provenance` / `--build-verification` evidence before installing missing remote-sync artifacts, and catalog/MCP read-only views classify provenance records and receipts (`list_provenance`, `search_provenance`). They remain evidence only: authority, policy, resource, transport, execution, and source-gate checks must still be supplied independently.

## Blob-ref job submission

Ref-backed jobs are exposed under `molten test job` so executable, input, output, and receipt identity is content-addressed before worker execution:

```sh
molten test job ref-submit --job-id job:echo --operation-id blake3:operation \
  --executable blake3:executable-manifest@12@elf-executable \
  --input blake3:input-manifest@1024@bytes \
  --authority-context-ref blake3:authority --policy-ref blake3:policy \
  --provenance-ref blake3:provenance --effect-ref blake3:effects \
  --out target/job-ref.submission.preserves
molten test job ref-execute target/job-ref.submission.preserves \
  --chunks target/chunks --ledger target/ledger \
  --receipt-out target/job-ref.receipt.preserves
molten test job status --ledger target/ledger --job job:echo
```

`job-ref-submission-v1` is content-ref-only and rejects inline executable/input bytes. The local deterministic worker reads and verifies chunk manifests before running the `local-echo-v1` handler, pins executable/input/output refs while active, emits fetch/verify/status/cleanup evidence, stores outputs as chunk manifests, and records `job-ref-receipt-v1` in the ledger. These receipts are execution evidence only; authority, provenance, policy, effect admission, transport, and resource trust remain explicit inputs.

## Job DAG Iroh worker UX

Recorded local-gossip worker execution is exposed after sync, target admission, and execution request construction:

```sh
molten test job worker-request \
  --admission-receipt target/job-admission.receipt.preserves \
  --execution-request target/job-execution.request.preserves \
  --authority-ref blake3:authority --resource-ref blake3:resource \
  --peer-bootstrap-ref blake3:peer-bootstrap --node-identity-ref blake3:node \
  --out target/job-worker.request.preserves
molten test job worker-run-local target/job-worker.request.preserves \
  --target-registry target/job-target-registry \
  --storage target/job-worker-storage --cache target/job-worker-cache \
  --admission-receipt target/job-admission.receipt.preserves \
  --execution-request target/job-execution.request.preserves \
  --transport-root target/job-worker-transport --ledger target/ledger \
  --out target/job-worker-run
molten test job worker-schedule-local target/job-worker.request.preserves \
  --target-registry target/job-target-registry \
  --storage target/job-scheduled-storage --cache target/job-scheduled-cache \
  --admission-receipt target/job-admission.receipt.preserves \
  --execution-request target/job-execution.request.preserves \
  --transport-root target/job-scheduled-transport \
  --coordination-authority-ref blake3:authority \
  --coordination-resource-ref blake3:resource \
  --coordination-policy-ref blake3:policy \
  --ledger target/ledger --out target/job-worker-scheduled
```

`worker-run-local` writes the worker request, remote dataspace envelope, publish/delivery receipts, replayable delivery log, assignment, status records, result, worker receipt, execution receipt, and output evidence. `worker-schedule-local` wraps that path with a coordination queue enqueue/dequeue, duplicate enqueue operation replay, lock/fencing-token lease acquisition, stale-token denial before worker execution, release, and a `job-worker-schedule-receipt-v1`. Worker transport, queue claims, lease tokens, and CLI receipts are evidence only; peer identity or message delivery does not grant authority, policy, resource, provenance, source-gate, or execution trust.

## Coordination control-plane UX

Coordination manifests and requests can be generated and applied without bypassing the control-plane state machine:

```sh
molten test coordination manifest --service queue \
  --policy-ref blake3:policy --resource-ref blake3:resource \
  --out target/coordination.manifest.preserves
printf '<item "job-1">' > target/coordination.item.preserves
molten test coordination request --service queue --operation enqueue \
  --key queue:jobs --client-session worker-a \
  --operation-id-ref blake3:operation --payload target/coordination.item.preserves \
  --authority-ref blake3:authority --policy-ref blake3:policy \
  --resource-ref blake3:resource --out target/coordination.request.preserves
molten test coordination apply --manifest target/coordination.manifest.preserves \
  --request target/coordination.request.preserves --out target/coordination-apply
molten test coordination show target/coordination-apply/report.preserves
```

`apply` writes `coordination-apply-report-v1` plus indexed request/receipt/state/assertion evidence. Duplicate operation ids replay the prior receipt instead of advancing state twice. Coordination receipts and reports are evidence only; they do not grant authority, policy, resource, transport, provenance, or source-gate trust.

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

For the current private OnixResearch git dependencies, the flake locks local Cargo checkout sources as `*-src` path inputs and unit2nix serves those checkouts to Cargo's git cache. This keeps the Nix builder from needing SSH access to GitHub. Latest local Nix nextest evidence: `nix build .#checks.x86_64-linux.nextest --no-link --print-out-paths --option eval-cache false --option substituters https://cache.nixos.org/ --option builders "" --option auto-optimise-store false --option min-free 0 --option max-free 0` -> `/nix/store/23b9r4wdy4s8nkzxpk78wsch82q3pxcw-molten-nextest`.

Strict Octet source-gate sequence:

```sh
cargo octet check --artifact-dir target/octet
cargo octet check -p molten --artifact-dir target/octet-lib -- --lib
cargo octet object corpus receipt \
  --output target/octet/object-corpus-receipt.json \
  src/artifacts.rs src/catalog.rs src/catalog_mcp.rs \
  src/coordination.rs src/delivery_idempotency.rs src/job_dag.rs \
  src/ledger.rs src/main.rs src/node_daemon.rs src/node_runtime.rs src/octet_gate.rs \
  src/operator_dogfood.rs src/plugin_host.rs src/preserves_rail.rs \
  src/protocol_session.rs src/provenance.rs src/raft_control_plane.rs \
  src/remote_dataspace.rs src/secrets.rs src/service_supervision.rs src/transcripts.rs
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

The strict gate is fail-closed: `warning-only` denies even when `cargo-octet` exits `0`, and `command.txt`, `status.json`, `summary.txt`, structured finding keys, object corpus receipts, and fingerprint evidence are bound by canonical refs in the Octet receipt. Current remediation snapshot: workspace and lib-only Octet are `clean` with 0 findings, 0 warnings, and 0 errors; focused object corpus has 1882 objects (`b3:a720bc13e9b97f46476f863be924cb84e3f4e5e160640a8695406b58f9227c33`); latest artifact import receipt is `blake3:911c54614efd88a0b30b9c8c1b0bbea14666fb52106f21f42141a1ae9a6b3e3e`, latest strict pass receipt is `blake3:ff518347c3607e3e9b43c0e65eda3d129e0108a559f32267078989a858e2f379`, and latest remediation plan receipt is `blake3:ce36126202930c7b25b47b2acafa877f109ecf35891db3df9946444c9eb81b48`. Caveat: this is configuration-clean with the broad high-noise lint families explicitly disabled in `dylint.toml`; source-remediated zero for those disabled families remains separate follow-up work. During warning burn-down only, use the explicit quarantine flow:

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

This runs a fresh local native two-actor suite through the Preserves harness rail and the in-process Molten runtime kernel, writes a canonical Preserves report with actor-registry, effect-log, policy-gate, capability-gate, budget-gate, admission-decision, executor hostcall boundary, and budget evidence, replays by injecting recorded clock/random effect responses to the same report/state refs, exports a sealed repro bundle with an embedded report gate receipt, and checks the report or sealed repro bundle as admissible pass evidence with `molten test gate check`. Harness steps now carry canonical Preserves values for message bodies, assertions, retractions, and exact-value observe patterns; string payloads are just the compatibility-friendly shorthand. Suites may include an optional static `<policy-v1 "molten.harness.policy.v1" [...]>` fixture with deny rules such as `<deny "producer" "assert" #f "service.ready" "producer cannot assert readiness">`, and must include explicit `<budget-v1 "molten.harness.budget.v1" ...>`, `<actor-registry-v1 "molten.harness.actor-registry.v1" [...]>`, and `<capabilities-v1 "molten.harness.capabilities.v1" [...]>` fixtures; actor registries bind ids to executor kinds without inferred actors or fallback execution, reviewed Steel hostcall actors may include explicit `<steel-executor-v1 ...>` source/callable/allowed-hostcall fixtures and execute in a reviewed Steel VM with canonical execution receipts, reviewed Wasm hostcall actors may include explicit `<wasm-executor-v1 ...>` module/WIT/allowed-hostcall fixtures validated with `wasmparser` and executed through a no-WASI Wasmtime hostcall shim with optional `molten.wasm.abi.v1` Preserves byte input/output, adapter/remote-proxy kinds require explicit executable preflight fixtures and verified transcript profiles before deterministic gates, capability grants such as `<grant "producer" "send" "consumer" #f>` deny by default when no matching grant exists, and omitted budget/actor/capability fixtures cannot execute or satisfy pass-evidence gates. Denied turns roll back and denied effects do not issue ambient effect requests. Before runtime turns or effects, the harness canonicalizes the policy, capability, and budget snapshots and emits `<policy-gate-v1 "molten.harness.policy-gate.v1" ...>`, `<capability-gate-v1 "molten.harness.capability-gate.v1" ...>`, and `<budget-gate-v1 "molten.harness.budget-gate.v1" ...>` evidence; policy gates now derive deterministic Nickel static source/export refs, validate a Basalt Nickel contract envelope/preflight receipt, and keep unreviewed Steel/dynamic predicate records fail-closed; capability gates validate a Basalt authority contract/preflight receipt, bind explicit empty local UCAN proofsets, and bind every grant ref used by admission authority evidence; budget gates derive deterministic Nickel resource-policy source/export refs and validate a Basalt resource preflight receipt. Report validation fails closed unless the embedded suite has explicit budget/actor/capability fixtures, report budget/actor-registry and policy/capability/budget gate evidence match the embedded suite, every declared actor has matching `<executor-preflight-v1 "molten.runtime.executor-preflight.v1" ...>` evidence with bound executor conformance suite refs, reviewed Steel actor fixtures carry `<steel-review-receipt-v1 ...>` source/callable/allowed-hostcall bindings and admitted Steel steps carry `<steel-execution-receipt-v1 ...>` VM/input/output evidence, reviewed Wasm actor fixtures carry `<wasm-inspection-receipt-v1 ...>` module/import/WIT/allowed-hostcall bindings and admitted Wasm steps carry `<wasm-execution-receipt-v1 ...>` fuel/memory/hostcall/ABI execution evidence, every observation has exactly one matching admission decision with authority evidence, the decision recomputes from embedded capabilities plus policy, actor activity is wrapped in matching `<actor-input-v1 ...>`, `<hostcall-request-v1 ...>`, `<hostcall-decision-v1 ...>`, and `<actor-output-v1 ...>` envelopes, denied turns contain only rollback evidence, and denied effects contain no effect records. Successful gate decisions emit canonical `<gate-receipt-v1 "molten.harness.gate-receipt.v1" ...>` Preserves artifacts containing artifact refs, validation/replay results, explicit-budget/no-default-resource-policy/actor-registry/no-inferred-actors/executor-boundary/executor-preflight/executor-conformance/Steel-review/Wasm-inspection/Wasm-execution/executor-hostcall-boundary/Nickel-policy-source/Nickel-export/Basalt-preflight/Basalt-authority/UCAN-proofset/Nickel-resource-policy/Basalt-resource/capability/hostcall/admission check evidence; without `--receipt-out`, the receipt is emitted on stdout. `molten test repro export --profile deny-sensitive` (the default) writes a sealed `<harness-repro-bundle-v1 ...>` refs artifact plus `gate-receipt.preserves`; the sealed bundle includes redaction policy/gate evidence and refuses sensitive Preserves marker records such as `<secret ...>`, `<confidential ...>`, `<credential ...>`, `<private ...>`, and unvalidated `<encrypted-ref ...>`. `--profile redacted-diagnostic` emits deterministic redaction markers plus canonical redaction transform manifest/receipt files and remains diagnostic-only: `repro verify` and `test gate check` fail closed unless a future policy explicitly marks the transform gate-preserving. `--profile encrypted-private` emits validated `<encrypted-ref-v1 ...>` placeholders and a private-bundle profile; unpacking requires passing reveal receipts via `--reveal-receipt`, while missing, malformed, or unauthorized reveal evidence fails closed. `molten test repro verify` emits canonical `<repro-verify-receipt-v1 ...>` receipts for sealed pass bundles, `molten test repro unpack` materializes verified report/suite/receipt contents or diagnostic redacted evidence, and bundle gate checks recompute redaction evidence and embedded report receipts before emitting a new `repro-bundle` gate receipt, while failure repro bundles remain diagnostic-only.

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
```
