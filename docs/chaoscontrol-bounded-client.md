# Bounded ChaosControl client boundary

This is the Aspen-owned boundary for [Aspen #40](https://github.com/brittonr/aspen/issues/40) and [ChaosControl #19](https://github.com/brittonr/chaoscontrol/issues/19). Molten is Aspen's internal codename; existing `molten` crate, CLI, schema and requirement identifiers are unchanged. Source fixtures and commands below are not recorded execution evidence. The native profile does not qualify a guest or close the upstream campaign.

## Admitted native profile

The public API is `molten::fabric_simulation::bounded_client`. `Input` schema `molten.chaoscontrol-client.v1` admits exactly the ordered participants node-a/node-b/node-c, one opaque control-plane request, zero application-payload bytes, six bounded coarse phases, and 1–32 recovery event-loop ticks per participant. Each phase requires its canonical phase-specific request identity and a shared consistency ordering key; a different well-formed content ref is not an admitted substitute. Input is limited to 16 KiB. Unknown schemas/profiles, custom payloads/topologies, extra operations, duplicate identities and prerequisite-breaking schedules reject before effects.

The input also binds the actual submit request/opaque command refs and the existing fixed termination budgets: 1,200 event-loop iterations per child and a 30-second child shutdown/readiness bound. Unsupported budget changes reject; these are not new fault-engine or timing guarantees.

1. Start distinct live children with existing admitted seven-port startup.
2. Elect node-a, submit the real request, complete a quorum read, create/install the snapshot for lagging node-c, and request the existing transport partition.
3. Checkpoint independently observed application, quorum, storage and participant state.
4. Kill and reap the owned children, retaining their Redb roots.
5. Reopen actual local state before peer ingress.
6. Stop disruptive faults, restart elections, observe stale-leader fencing, and cover each participant's finite recovery horizon before clean termination.

These are explicit coarse operations, not an implementation of arbitrary submit/read/fault scheduling. The driver reuses the existing live-process Raft child and the production admission, replication, application, Iroh and Redb adapters. `WorkloadExecutor` returns milestone outcomes; those outcomes alone never pass the independent semantic checker.

## Observation and property boundary

The independent checker compares actual request application counts, acknowledgement retention, validated membership/quorum receipt lineage, local reopen provenance and per-participant term advancement. Application counts come from the actual handler's request inventory, not expected result arrays. A native receipt is parsed and hash-bound before normalization; arbitrary callers supplying normalized Rust facts have not established live execution.

Transport fault intent is a requested phase. Each follower emits a separate application marker only when its existing fault gate drops an actual received event. Crash application is retained only after the owned process was killed and reaped. A missing marker is not fault coverage. The fault-off phase requires the partition to be absent and every participant to emit its own recovery horizon. Each interval starts only after that participant advances beyond its independently observed reopen term with the disruptive fault off; a fault during observation resets the interval. The version-2 horizon marker binds the reopen term, first/last loop positions and observed terms, and fault-off status. The checker independently validates the duration, advancement at interval start, reopen binding and finite loop bound, and uses the interval's end term rather than a later terminal election. Missing intervals or terminal observations remain incomplete, not completed recovery or semantic failure. Ticks are native event-loop iterations, not wall time, guest instructions or a KVM horizon.

Native storage evidence reports process-visible readback and process-crash recovery. It does not infer observed sync or physical power-loss durability from successful reopen. The oracle uses the existing storagefault classifications and sticky health: applied, synced, outcome unknown and corrupt/quarantined observations remain different, and a successful retry does not reconcile uncertainty. Machine-loss requests require synchronization observations. Existing owner reconciliation remains responsible for clearing quarantine.

A local recovery record is captured before peer ingress and bound to the actual owner recovery receipt, completed request, restored or replayed application and nonempty durable inventory. Local byte provenance does not assert voting admission: an owner can restore its local snapshot while still requiring the independent peer-query rejoin protocol. Peer repair requires explicit source/receipt evidence; eventual success is insufficient. An empty local inventory is local absence, never evidence of global absence. Native execution here exercises local recovery; peer, global-absence and incomplete examples in the oracle bank are modeled checker fixtures only, not campaigns over those conditions.

## Intentional semantic control

`duplicate-application` is a test-only defect, not an allegation about production. After an admitted commit it deliberately bypasses the application adapter's apply-once guard and calls the real application handler again with that committed command. Separate deployment and reached files bind the input and intended `molten.client.apply-once.v1` property. Reached is emitted only after the handler inventory contains the duplicate. The intended independent property must reject that divergence in the same history. Suppressing it invalidates qualification rather than counting detection elsewhere.

Healthy, reached/detected, not reached, reached/survived, invalid, unsupported and incomplete remain distinct. Pure outcome fixtures additionally cover no-op, missing participant, unavailable end/readiness/recovery, short horizon, stalled participant, fabricated/mismatched receipt, absent acknowledged state and storage uncertainty. A no-op can return Delivered through WorkloadExecutor and still lacks the observations required to pass.

Reduction reruns the same binary against successively shorter valid prefixes. It preserves every participant and prerequisite, requires the same property identity, and retains both original and reduced canonical input. A prefix's still-running owned children are reaped before observation collection; omitted phases are not executed to manufacture missing evidence. Pre-checkpoint prefixes have no participant samples and remain incomplete rather than reading nonexistent checkpoint receipts. A reduced prefix can expose a safety failure without becoming a completed recovery qualification.

On Linux the driver measures `/proc/self/exe` once per native comparison case and reuses that immutable running-image identity for independent producer sets and valid-prefix reduction. It launches the same running image rather than reopening `current_exe()`'s pathname. Replacing or unlinking a Cargo output cannot switch a later prefix to another binary or make original-image readback disappear. Other platforms recheck the selected path and digest before each run. Independent repeats still launch fresh child processes and produce their own observations; only the unchanged executable measurement is shared. This is native process identity preservation, not saved-snapshot replay or a guest qualification claim.

## Native commands and retained artifacts

Run only after all implementation slices settle, from the repository's pinned Nix development shell. Select a new, nonexistent retention directory. Bind SOURCE_REVISION and SOURCE_REF to the exact immutable source actually built; missing values are recorded as unavailable and do not create an original-build provenance claim.

```sh
nix develop -c cargo test --lib fabric_simulation::bounded_client -- --nocapture
MOLTEN_CHAOSCONTROL_SOURCE_REVISION="$SOURCE_REVISION" \
MOLTEN_CHAOSCONTROL_SOURCE_REF="$SOURCE_REF" \
MOLTEN_CHAOSCONTROL_RETAIN="$OUTPUT" \
nix develop -c cargo test --lib fabric_consistency::raft::live_process::bounded_client -- --nocapture --test-threads=1
```

The native tests retain healthy, original, independent original-repeat and each attempted reduction-prefix directory. Retained artifacts include canonical input; actual checkpoint/reopen owner receipts and samples; participant horizon, applied-fault, crash, deployment/reached markers; execution outcomes; oracle classification; immutable executable hash and source/adapter/oracle/schema/control bindings; suppression control; valid-prefix reduction; and child logs. Child logs are diagnostics, never qualification authority. The repeat uses fresh child processes after the original producer children exit and compares exact build/input/property bindings. It is native independent rerun, not ChaosControl saved-snapshot replay. Repaired candidates must get new source/binary bindings and the same semantic scenario plus fresh nearby recovery budgets; old instruction coordinates cannot be transplanted.

The parent subsequently ran the bounded-client selection (15 tests passed) and
a retention-enabled native six-test run (all passed), including healthy,
duplicate-application, independent-repeat and real prefix-reduction paths.
Canonical artifacts are retained under the change's
`evidence/native-2026-11-13/`. Source-revision environment inputs were unavailable
in that local run; this does not establish immutable-source or guest/campaign
qualification. Final integrated source/lifecycle checks remain separate.

A subsequent six-test run on 2026-11-15 used the realized immutable Nix test
image and passed healthy, deliberate duplicate detection, independent repeat,
valid-prefix reduction, receipt-tamper and recovery-horizon controls. The
[invocation and build recipe](../.cairn/changes/add-bounded-chaoscontrol-client/evidence/native-2026-11-15/invocation.json)
bind the exact source store path, BLAKE3 of its NAR bytes and actual executable
hash; the retained canonical build records contain those same source and
binary identities. This closes the native source-binding gap only. Guest,
multi-guest transport/storage and independent saved-snapshot campaign/replay
qualification remain blocked by the prerequisites below.

## Guest handoff through existing mechanisms

The first requested executor profile is single-vCPU per guest, not SMP. Use the existing ChaosControl Rust guest image/process bundle and `chaoscontrol_smr::observation::guest::MultiNodeGuestIntake`, not a new control service or qualification report. At reviewed ChaosControl source ec8aa799bad4735bf1028a092ca39feb08fb4bb7 (issue baseline), GuestSource binds guest_id, executable_ref, source_revision_ref, adapter_ref and launch_contract_ref. The intake requires at least two distinct guest IDs and exact participant-source membership. Its shell must independently remeasure retained artifact bytes, emit actual protocol observations plus complete drains, and run the existing cohort/history validators. Those validators do not attest to execution or grant replay promotion.

Carry exact client source, binary, image/initrd, kernel, complete build/runtime closure, adapter, independent oracle, property catalog, generator and workload schema identities into that existing evidence path. `guest_dependency_blockers` checks identity availability and the single-vCPU prerequisite only; empty blockers is preflight, never guest qualification. The existing root Cargo/Nix pin b8c440ea3b19df796542e58e8ee36200e1c3db85 supplies snapshot descriptors only; it is not a client SDK/executor pin and is not silently upgraded here.

### Exact remaining prerequisites

- No measured Aspen client guest binary/image/kernel/dependency closure has been produced. The existing native child is test-only and depends on Tokio process/thread/clock/entropy support, Redb filesystem/sync behavior, Iroh loopback/peer networking, the test executable and loader/runtime libraries. The Nix test-binary derivation already exists as `moltenTestBinaries`, but it is not an admitted guest.
- The native three-child signal directory does not become a cross-guest control protocol. The current guest process supervisor admits memory-backed shared directories; block-backed shared directories are explicitly rejected pending reviewed device mapping. Do not relabel tmpfs as persistent storage or pretend that one guest satisfies MultiNodeGuestIntake's distinct-guest requirement.
- ChaosControl #19 must supply the admitted single-vCPU-per-guest topology, actual fault application and supported persistent-device/transport handoff for this client, and retain the source/build/closure/observation/horizon evidence. #14/#15/#18 own retained finding/replay, mutation corpus attribution and replay-ready handoff.
- Actual SDK instrumentation quality, guest execution, producer-exit saved-evidence independent replay and qualification must run through those existing rails before cohort acceptance. Missing dependencies and unsupported profiles remain blockers, not waived tests.

No root Cargo, lock, build plan or flake edits are needed for the native boundary. Parent documentation integration should link this page from README and the documentation map; do not add guest package aliases to unbuilt native fixtures. An actual guest cutover requires an immutable upstream source contract and real Nix closure packaging under the existing pin-update policy.

## Non-claims

This bounded projection does not prove arbitrary Aspen operations, content-bearing application recovery, linearizability, exactly-once external effects, physical power-loss behavior, deterministic KVM/native parity, SMP support, snapshot-backed replay, production readiness or release eligibility. It does not satisfy object-store/provider uncertainty owners #13/#14, capability qualification #22 or storage-denial execution #24.

The recovered source working copy may be on tmpfs. That source location is not NVMe or machine-loss durability evidence; native tests still report only their actual selected process-crash/storage scope.
