# Native Molten integration boundary

The reviewed delivery contained a guarded reference-scheduler patch and a separate
seven-crate batch service. Molten adopts only the standalone service here. The
original reference patch is validated in an isolated worktree at its exact base;
it is not applied to current mainline.

The pilot is NOT registered in `system_extension::Dispatcher` and does not change
the root Cargo workspace, ALPN registry, or any admitted system-tier manifest.
It does not produce Basalt/Cairn admission receipts. Its deployment profile is
`standalone-single-controller-import-free-component-batch-v1`, with its own operator
policy and controller/worker databases. Molten contributors maintain this pilot.

## How the boundary should be connected

| Existing owner | Pilot port | Integration law | Required follow-on gate |
|---|---|---|---|
| System-extension lifecycle and generation | `Controller`, `WorkerService` | Admit the exact manifest, service identity, active generation, resources and port bindings before callbacks or effects. | A separate installation change must cover denied activation, stale generations, drain, recovery and teardown through `system_extension::Dispatcher`. |
| Fabric membership and extension-owned placement | `PlacementPort`, controller assignment input | Use observed descriptors and exact epochs; advisory placement feeds one authoritative reservation transaction. | Reject stale membership, unavailable capacity and competing reservations before dispatch; do not infer admission from connectivity. |
| Assignment persistence and consistency | `StateStore`, `WorkerJournal` | Workload insertion, reservation and allocation ownership share one owner; persist intent before effects and retain unknown outcomes. | Prove concurrent reservation exclusion and crash/reopen reconciliation against the selected owner; two successful independent writes are not atomicity. |
| Coordination delivery | `WorkerClient`, assignment outbox | Preserve exact attempt fences and uncertain results; delivery acknowledgement is not execution completion. | Exercise duplicate/delayed start, cancellation tombstones, uncertain delivery and restart without a second launch. |
| Content storage and replication | `ContentPort`, `IdentityPort` | Reuse verified bytes and manifests through explicit identity conversion; standalone IDs are not native object IDs. | Positive conversion and negative substitution/format cases must pass under current content-read authority and publication eligibility. |
| Basalt, provenance and resource admission | `AdmissionPort` | Recheck current capability, provenance, policy and resource facts before protected effects. | Deny missing, revoked, stale and wrong-generation grants before storage or execution; identity and authenticated peers grant no capability. |
| Execution and supervision | `ExecutionPort` | Bind the exact runtime profile, limits and teardown evidence; in-process recovery does not establish native/VM cleanup. | Verify cancellation, exhaustion, restart and owned teardown for the chosen backend before any wider runtime profile is admitted. |
| Evidence and bounded observability | `ObservabilityPort` | Emit canonical evidence at semantic boundaries and bounded read-only status; counters are not authority. | Bind receipts to the exact cohort and reject crossed identities and overclaims; logs cannot override deny receipts. |

The orchestration semantic core should remain an extension-owned crate. Its
commands, attempts, DAG behavior and retry policy must not move into node-core
semantics. Infrastructure adaptation should not redefine those decisions.

## Unimplemented native/production integration

Actual dispatcher installation and callback routing, canonical fabric-port
registration, Basalt/Cairn integration, shared reservation storage,
coordination-delivery composition, root content-ID translation, live consistency
for controller failover, persistent services and production qualification are
separate changes. None is implemented by this adoption. The standalone service is
not an implicit fallback for any of them.

Root baseline for the companion source patch:
`bb6f3830ee7327da9875ea85a8c8e25697eddc35`. The guarded installer refuses another
commit or changed source blobs. Reconcile a later branch explicitly rather than
removing the guards. No GitHub write was performed for this delivery.

Current mainline uses `DistributedSchedulerOperation::Complete::assignment_token`,
derived from job, owner and lease epoch, and rejects a stale token before completing
the job (`crates/molten-core/src/fabric_simulation/reference.rs`). Replacing that
contract with the historical overlay's `lease_epoch` field would be a regression in
API compatibility, not a required pilot dependency. The guarded installer remains
unchanged in the source bundle. Its default mode is the dry-run check; the delivered
CLI has no `--check` flag.

Passing this pilot does not prove Wasmtime, component, worker, transport,
cross-host or whole-system correctness, production isolation, controller HA,
exactly-once external effects, or release eligibility.
