# Closure repair design

## Boundary

Repair operates at the canonical-content boundary: verified blobs and manifests whose identities are already established. A valid content hash proves which bytes were received; it proves nothing about which head is current. This change keeps those mechanisms separate.

## Proposed decision

The pure repair planner works on a required-closure set and a bounded inventory of per-replica, per-object verification facts. Donor eligibility is per object: a replica is a donor for exactly the required objects it holds with a passing verification result. The planner emits a bounded repair plan of per-object transfers with chosen donors and a completeness decision.

Completeness is explicit:

- Every required object has at least one intact verifiable copy: plan is complete.
- At least one required object has none: plan is a typed incomplete repair. It may still include transfers for the objects that do have donors, and the outcome names the unrecoverable identities.

Verification stays on the receiving side: assembled bytes are verified against the established identity before they enter the repaired set, and a verification failure on one object never poisons others. No new content identities are created by repair; repair recovers bytes matching known identities only.

Resource and retention admission, epoch fencing, protected content, and fault-domain placement rules remain as they are. The scenario assumes explicitly stated recovery and resource conditions; it grants no new capacity or authority.

## Simulation and fixtures

The reference simulation exercises the scenario with bounded inventories: several replicas, each with a distinct corrupted or missing subset. Negative fixtures cover zero-intact-copy objects, identity mismatch on assembly, donor flapping between plan and transfer, and unbounded inventory rejection. The scenario never asserts that repaired bytes make any replica current placement or authoritative.

## Compatibility and coordination

Operator status gains a completeness field on repair outcomes; existing complete-repair outcomes keep their meaning. `repair-lost-replicas-after-prior-success` remains the owner of stale-success reuse; its F04 regression must keep passing unchanged. No shared generic repair abstraction is introduced.

## Validation and ownership

Content-replication maintainers own planner, verification transitions, fixtures, and docs. Run the existing replication baseline before edits, add the closure scenario to normal repository tests with named identities and bounded inventories, and retain the bytes-not-authority non-claim. Run focused Octet and Clippy checks, workspace tests, relevant Nix checks, and required Cairn gates. This plan grants no implementation permission.
