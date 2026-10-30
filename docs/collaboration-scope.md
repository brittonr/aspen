# Collaboration-scope implementation boundary

The collaboration domain is separate from fabric node membership. Its pure records and transitions live in `crates/molten-core/src/collaborationscope`; canonical encoding, authority acquisition and effects live in `src/collaborationscope`. The public facade is `molten::collaboration_scope`.

## Mechanisms and admission

`collaboration_scope::admit_extension` uses ordinary system-tier and exact fabric-port admission. The collaboration schema is `molten.collaboration-scope.v1`; the required port classes are consistency, durable state, time, authority, resources and evidence. Artifact possession or a successful callback does not activate the service.

`Installation::from_host` requires a running host and the exact active consistency group, service generation and application manifest. `current_scope` requires a linearizable control-port read, the actual Raft ledger's matching quorum-currentness record, the current term and applied index, and the admitted logical-time profile. The first profile uses committed consistency events as logical positions. Membership expiry is expressed in those event positions, not elapsed wall time or process timers. A currentness observation is consumed once per installation and bounded collection exhaustion fails closed.

`collaboration_scope::verify_decisions` calls the pinned Basalt and UCAN implementation at revision `89675cd4f585f837323c049e4a25f7b94c903038`. It verifies token signatures, the declared trusted issuer and holder, delegated resource and ability, policy intersections, the exact holder-signed operation binding, caveats, revocation and replay through supplied admitted verification services. `NonceLedger` implements the real verifier's replay port using the admitted durable-state adapter. Tokens, DIDs, signatures and policy bodies are borrowed at this boundary; decision projections retain only nominal references and the verified invocation digest. The operator must supply the trusted issuer/holder and current revocation/consent observations through its admitted authority composition, not from untrusted request metadata.

`collaboration_scope::Application` implements the actual consistency service's `CommittedBatchHandler`. Commands are staged only after real authority acquisition, durably stored and published through the admitted callback-value port. Replicas materialize committed command identities through that port. Scope membership, resource binding, operation-result identities and bounded commit receipts are persisted through `RedbDurableStateAdapter` atomic batches with machine-loss durability. Storage uncertainty blocks serving; it is not converted into policy success. Exact operation replay retains the original result identity.

An ordered stale or conflicting command produces a closed denial outcome. Scope and resource records remain unchanged; only the consistency application position advances, so an invalid request cannot wedge later valid ordered commands. The pure transition itself still preserves its entire input on denial.

The native executable is `molten-collaboration-scope`. It speaks the existing materialized native callback protocol and persists a bounded bridge record across Initialize, Checkpoint and Recover. Bridge metadata records generation, incarnation, sequence and the last proposal intent, not membership or committed authority. A request emits the exact typed `commit-staged` effect. `native_runtime::admit` checks the measured complete runtime configuration against the actual running host. The public host separately checks independent current `install-effects` and `route-effect` credentials before creating resources and routing effects; source provenance grants neither permission.

`native_runtime::Worker` retains actual Iroh, Raft and machine-loss Redb adapters on its owning worker. Before proposing, it reacquires real scope UCAN/Basalt authority from independently provisioned operator roots, obtains an actual linearizable read and a distinct durably reserved quorum confirmation, and requires unchanged state, term, applied position, logical time and revocation facts. Historical decisions and untrusted prepared intents are not grants. Holder replay and probe reservations use the admitted `NonceLedger` mechanism. The private queue adapter returns no completion success. The public effect completes only with the actual applied/committed request position, a matching Raft commit-quorum ledger record, the exact machine-loss durable operation receipt and canonical state identity. A committed closed denial remains a denial in that receipt; work completion is not mutation authority. Uncertain delivery is not retried or reported as committed. Native execution remains trusted rather than sandboxed.

## Consumer and confidentiality boundary

Snapshots bind the exact scope revision, membership epoch, resource binding, audience, ability, request, policy intersection, consent reference, revocation observation, service generation, engine epoch and canonical state. Sharing and moving have distinct abilities; read/use decisions do not satisfy them. Explicit moves replace one resource-map entry and require both source and target policy admission.

`decode_snapshot`, state decoders and command decoders accept closed bounded records, not arbitrary metadata bags. Unknown fields, unbounded diagnostics and raw values in nominal-reference positions are rejected. `consumer_value` provides metadata-only Lattice effect-attempt, Animus context-reference and Tile read-only views. Consumers must obtain current admission before a protected effect; these projections are neither reusable authority nor membership stores.

The implementation does not establish identity truth, global revocation freshness, Basalt/UCAN correctness, transport confidentiality, external-effect safety, prior-disclosure erasure, exactly-once effects or release eligibility. Native execution remains trusted, not sandboxed. Existing experimental Raft production-admission flags remain unchanged and false.

## Reference and evidence status

The QM requirements reference is commit `7f2c916360f1797a8ff2a77ce2ce40c5fabab087`. Its immutable README and SECURITY document informed personal/shared scope isolation, organization-floor narrowing, explicit resource ownership and non-transitive sharing. Its PostgreSQL layout, TypeScript implementation, bearer app links, credential materialization and model-screening mechanisms were not adopted. The immutable LICENSE declares MIT, copyright 2026 QM contributors; no QM implementation was copied and no QM tests were run.

The named prerequisite owners exist in the archive: fabric consistency (2026-07-24), durable-state ports (2026-07-12), whole-system simulation (2026-08-01), system-extension runtime (2026-07-11), and nominal authority/artifact references (2026-08-24). Their archive presence does not admit a new consumer or transfer execution claims. Current actual source, profiles and ports must still pass admission.

Core behavior, actual cryptographic/durable shell tests, an installed-profile deterministic simulation and a genuine three-process Iroh/Redb collaboration scenario are authored. The simulation uses the maintained `Application`, real verifier, simulated durable-state/transport adapters, shared scheduler, typed exact installed-workload selection and independent domain invariants. It explicitly replays recorded committed-log delivery; it is not live consensus or disk evidence. The distinct-process rail separately exercises actual election, concurrency, partition, snapshot catch-up and recovery. The generic installed-world prerequisite preserves reference-world admission and its original semantic-oracle requirements; canonical profile-body admission and node/generation/request/invariant substitution mutants are authored.

No build, test, formatter, linter or execution rail has been run for this source cohort. Genuine installed-host acceptance must execute the source-gated retained public host with independently signed operator and scope credentials and a parent-produced immutable native executable. Authored adapters, fixtures and test names are not runtime evidence. Lifecycle tasks remain unmarked until the parent runs the actual rails and records their output; experimental production-admission flags remain false.

Targeted parent-owned execution commands are:

```sh
cargo test -p molten-core --lib collaboration_scope::tests
cargo test -p molten --lib collaboration_scope::tests
cargo test -p molten --lib collaboration_scope::simulation
cargo test -p molten --lib fabric_consistency::raft::collaboration_process::three_participant_conflict_partition_restart_snapshot_and_currentness_recovery
```
