# Bounded content replication

Molten implements content replication as an optional system extension. Ordinary content reads and writes do not activate replication.

## Ownership

The extension owns replica policy, placement, repair, handoff, and convergence status. Content stores retain byte identity, verification, transforms, protection, and local availability.

Fabric adapters retain transport, time, membership, placement, durable-state, resource, identity, and observation mechanisms. These adapters do not define replica policy.

The content-replication maintainers own the pure core in `crates/molten-core/src/content_replication/`, the shell and adapters in `src/content_replication/`, and their conformance evidence.

## Pure planner

The functional core receives a complete `ReconcileInput`. This input contains the manifest, inventory, peers, operation history, and observed tick.

The manifest binds these facts:

- the service identity and generation;
- the membership and placement epochs;
- the content and transport profiles;
- the replica, repair, retention, resource, and evidence policies;
- all required fabric ports;
- the fixed non-claims.

The planner returns ordered transfer, repair, handoff, defer, and cleanup actions. It performs no file, network, clock, or process operation.

Operation identity includes the content and manifest references. It also includes source, receiver, generation, epochs, action, transform, protection mode, and attempt.

Current replicas must match the active generation and epochs. Stale replicas can supply verified bytes, but they cannot satisfy current placement.

Current verified counts come from the supplied inventory, not prior transfer outcomes. A new reconciliation after same-epoch loss selects a fresh operation at the next attempt for that content and receiver in the current generation and epochs, across prior outcomes and action kinds. Exhausted attempts retain unresolved demand. Exact saved-operation replay preserves the original outcome without another transfer; it is not a new reconciliation or evidence of present availability.

Each plan retains the missing replica count per content independently of its actions. Status clears a deficit only when enough distinct selected receivers have verified results matching the exact operation, content, source, receiver, generation, epochs, and attempt. Missing or malformed result references and duplicate operation identities cannot supply success. A successful subset cannot fill unselected demand or another content's deficit.

Cleanup checks the cumulative remainder after every selection against desired replicas, minimum verified replicas, and minimum fault domains. It skips unsafe, pinned, or uncleared candidates and considers deterministic safe alternatives. If no safe alternative exists, the excess remains and the plan is partial.

## Closure repair

Closure repair works at the canonical-content boundary: verified blobs and manifests whose identities are already
established. The pure repair planner takes a required-closure set and a bounded inventory of per-replica, per-object
verification facts.

The Rust facade exposes `ClosureInput`, `ClosurePlan`, `Completeness`, and `plan_closure` from
`molten_core::content_replication` (also available under its public `repair` module). Owner-repeating repair names
are removed rather than retained as aliases. Identity framing errors propagate as the existing domain `Issue`;
the planner never substitutes a zero identity for a failed canonical frame.

Donor eligibility is per object. A replica donates exactly the required objects it holds with a passing verification
result, so a cohort whose members each hold a different intact subset repairs the whole closure even though no single
member holds all of it. The plan is bounded, and each transfer names the object and its chosen donor.

Completeness is explicit. When every required object has an intact verifiable copy the plan is complete. When one does
not, the plan is a typed incomplete repair: it still carries the transfers for the objects that do have donors, and it
names the unrecoverable identities. Verification stays on the receiving side: an assembled object is verified against its
established identity before it enters the repaired set, and a mismatch rejects that object alone. A donor that flapped
between the plan and the transfer is refused at transfer time.

Repair recovers bytes matching known identities only. It creates no new content identity, it grants no capacity or
authority, and repaired bytes never make a replica the current placement.

## Publication eligibility

A profile states its own durability requirement before the metadata publication
becomes authoritative. The requirement is the profile's replica policy, and the
gate compares it against the admitted storage observations: the exact content
identity, the publishing operation, the receiver set, the retention generation,
the placement epoch, and the storage guarantee kind. A replica count alone never
satisfies the gate. The receipt names the storage guarantee, so a local
guarantee never satisfies a profile that asks for replicated availability across
fault domains, and a receipt that omits a binding is refused.

The gate sits between the transfers and the metadata publication. A local
development profile and a replicated deployment therefore declare different
durability explicitly, and the receipt is part of the published evidence.

The Rust facade exposes `PublicationObservations`, `PublicationReceipt`, `PublicationIssue`, `check_publication`,
and `identify_publication_receipt`. The identity function returns `Result<String, PublicationIssue>` and reports
`ReceiptFieldTooLong` if a field's byte length cannot fit the canonical `u64` frame; the gate propagates that issue
instead of hashing a sentinel length. These source API names replace the former eligibility-prefixed names without
aliases. Canonical identity contexts, field order, bytes, and publication authority rules are unchanged.

## Imperative shell

The shell observes authority, identity, membership, placement, time, inventory, and durable history before it runs the planner.

For each transfer, the shell uses this order:

1. Acquire the retention pin.
2. Request the exact receiver-owned operation.
3. Validate the transport envelope.
4. Verify the content through the content port.
5. Store the terminal operation.
6. Publish the operation observation.

The shell stores and publishes status after all actions. It publishes the aggregate receipt last.

Invalid planning inputs reject before resource reservation, durable writes, or publication. Verification and envelope rejection do not append a terminal success or publish completion. A receipt is complete only for a ready plan with no remaining deficits, active operations, or failures; unresolved cleanup also remains partial.

Cleanup requires the content rule authority and a matching cleanup clearance. An active retention pin prevents cleanup.

The public reconciliation API always observes current facts and replans; `last_plan_ref` and historical plans do not authorize execution. Every cleanup execution rechecks retention admission even when an earlier cleanup operation succeeded. A malformed result returned after deletion is post-effect uncertainty, not proof that deletion rolled back.

## Canonical compatibility

New plans use `content-replication-plan-v2` and the `onixresearch.molten.content-replication.plan.v2` identity context. They bind ordered per-content `replica-deficit` entries as well as actions, so action-free plans with different unmet demand have different identities. Active `reuse` actions are removed; fresh repair uses the existing bounded attempt field.

The operator status record moves to `content-replication-operator-status-v2`: it carries the repair completeness, the
unrecoverable identities, and the repair non-claims. Operator status is a bounded view, not a durable authority
record, and no reader of the v1 shape exists.

Operation/action identity framing and operation, status, and receipt v1 record shapes remain unchanged. Canonical status admission rejects invalid bindings, inconsistent counts, invalid or duplicate references, missing non-claims, and exceeded reference bounds. Historical canonical bytes and identities are not rewritten. An old receipt or unsafe plan remains historical evidence only; it cannot bypass corrected current planning or deletion admission.

## Conformance

The deterministic profile uses the accepted simulated content, transport, and durable-state adapters. Fault tests cover cancellation, partition, timeout, unavailable peers, corruption, and crash-before-mutation.

The live loopback profile uses the existing Iroh transport adapter and capability-rooted local content adapter. Both profiles run the same planner and shell.

The multiprocess adapter sends exact content bytes through two child processes. The request reference is the planned operation identity.

The transport harness indexes the request and payload inputs. Offline verification binds the parent receipt, child receipts, payload, and operation request.

## Operator status

The bounded operator view reports these facts:

- desired and verified replica counts;
- the placement epoch and active plan;
- under-replicated content;
- active operations and transfer resources;
- failures and retention pins;
- the closure repair completeness, the unrecoverable identities, and the repair non-claims;
- evidence references and non-claims.

The repair completeness is explicit. A complete repair means every required object has an intact verifiable copy
somewhere in the cohort. An incomplete repair names the unrecoverable identities and keeps the progress on the
objects that did verify. The view carries the bytes-not-authority claim: recovered bytes are bytes matching
already-established identities, so they never make a replica the current placement and never decide which head is
authoritative. A view without that claim, with a completeness outside the closed vocabulary, or with an incomplete
repair that names nothing is refused.

The view contains no content bytes, credentials, private keys, transport handles, or backend objects.

## Non-claims

A replica count does not prove permanent durability. A local repair does not prove global availability.

Transfer completion does not prove exact-once delivery. Replication does not grant installation, execution, publication, merge, or deletion authority.

Replication preserves protected content. It does not decrypt or reveal that content.
