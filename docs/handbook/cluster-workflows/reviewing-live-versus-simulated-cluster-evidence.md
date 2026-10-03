# Reviewing live versus simulated cluster evidence

Mode: Review checklist

Use this checklist before accepting a cluster result in a change or release review. The deliverable is a claim-specific decision with linked evidence, not a single green label for every tier. This page is source-checked, not executed verification. The [distributed testing guide](../../distributed-testing.md) governs evidence scopes; the [VM fault companion](../../technical/simulation/vm-fault-evidence-boundary.md) explains why real platform execution still has narrow limits.

## 1. Write the claim before examining the result

- [ ] Is the requested claim a model invariant, local process lifecycle, platform integration, or actual cross-node transport behavior? Record the exact operation, topology, and fault boundary.
- [ ] Is the submitted artifact family appropriate? A local `cluster-harness-run` is not a simulation trace or VM test run, even if all contain two node identities.
- [ ] Are source/package/test identities and the intended fixture available? Record actual references supplied by the producer; do not substitute placeholder refs or a current working tree for the reviewed candidate.
- [ ] Does the review explicitly exclude authority, policy, deployment, and production claims not independently supported?

A useful acceptance note says which workflow and environment were observed. “Cluster works” is too broad to test against the evidence and obscures unavailable stages.

## 2. Audit simulation as simulation

- [ ] Are topology, scheduler profile, seed, commands, fault plan, child workflows, events, and final-state refs present for the submitted distributed simulation profile?
- [ ] Are virtual-time and ordering inputs explicit rather than inferred from host clocks, process IDs, sockets, or ambient randomness?
- [ ] Does replay evidence correspond to the same admitted experiment, not just the same seed or eventual state?
- [ ] Are negative cases relevant to the change—stale evidence, wrong topology, unauthorized transport, or duplicate semantic work—linked rather than replaced with a log saying the run completed?
- [ ] Are replay status and diagnostic-only repro scope preserved during export?

The [simulation inputs companion](../../technical/simulation/deterministic-simulation-inputs.md) is useful background, but a model result is not evidence that real I/O executed. Reference equality between declared simulation and live components also does not prove equivalence of their shells.

## 3. Audit the local process tier

- [ ] Was the checked fixture's ordered membership retained, including distinct state-root and logical transport handles?
- [ ] Does the directory's offline assessment match the preserved index and companion, and are unresolved diagnostics recorded?
- [ ] Are attempted phases backed by process receipts, with configuration, identity, startup, workflow, heartbeat, health, control, and shutdown evidence captured per node?
- [ ] Is cleanup supported by actual stop process and node shutdown artifacts, not just a populated summary field?
- [ ] Is the experiment described as bounded local integration rather than live network execution?

The [runner](../../../src/cluster_harness/parts/runner/p000/body.rs) invokes separate local child phases. Its one-request workflow bound is not evidence that a cross-node application request was sent. Its normal reverse-stop path requires aggregate startup success; [cleanup construction](../../../src/cluster_harness/parts/runner/p001/body.rs) does not justify inferring individual stop execution after partial startup. Record such uncertainty rather than overriding it with a parent pass-looking field.

## 4. Demand executable VM evidence for platform claims

- [ ] Is host support recorded as supported for the required capability, rather than unavailable, skipped, or denied?
- [ ] Do fault descriptors bind topology, target node/link, command profile, expected outcome, bounded duration or trigger, and preflight refs?
- [ ] Do passing fault receipts bind pre-fault observations, injection, child workflow, and post-fault evidence?
- [ ] Are logs retained as diagnostic adjuncts without replacing any required child refs?
- [ ] Does every shard preserve its actual scope: `fixture-metadata`, `executable-vm`, `aggregate-index`, or `diagnostic-only`?
- [ ] Does an aggregate merely index children, or does the review incorrectly treat that index as another executable pass?

The [VM validator](../../../src/nixos/vm/parts/validation/p002/body.rs) checks topology membership and ref relationships, rejects unavailable pass claims, and diagnoses missing injection or child refs. These are necessary checks, not proof that every supplied ref denotes a physically successful intervention. Review the producing workflow as well.

## 5. Verify the live exchange boundary

- [ ] Does the live transport gate bind sender, receiver, expected peer, topic, operation ID, ticket, peer admission, authority, send, receive, ingress, queue, dispatch, reconcile, acknowledgement, and protocol-gate refs?
- [ ] Can each referenced artifact be associated with the same intended exchange rather than an unrelated successful example?
- [ ] Was test-driver artifact copying performed only as export plumbing after the exchange, rather than substituted for a receiver observation?
- [ ] If a fault needed network control, is its support probe and actual intervention evidence available?

Do not transfer transport observations into current authorization or infer exactly-once application effects. Raft remains a control-plane concern, not proof about ordinary actor traffic; OpenRaft is not selected by this project.

## 6. Work a mixed-evidence review case

Suppose a submission contains a passing deterministic partition scenario, a complete two-node local harness export, and a VM partition receipt with unavailable network control. The requested claim is “the live two-node service tolerated the partition.”

The simulation can support the modeled partition invariant. The local export can support its lifecycle integration claim if its artifacts verify. Neither supplies the missing live intervention. The [fault contract](../../nixos-vm-executable-faults.md) requires unavailable support to remain unavailable or denied, and the validator rejects unavailable-as-pass. The correct review records useful lower-tier evidence while withholding the live fault-tolerance claim. Do not rerun until green or relabel an aggregate as executable evidence.

## 7. Close the review with a bounded decision

- [ ] Are unresolved source-review gaps, unavailable execution, stale refs, and undeclared variance visible?
- [ ] Does release evidence respect the zero-retry rule or include a separately accepted remediation boundary?
- [ ] Have private attachments and failure bundles received their own reveal/redaction review?
- [ ] Does the final decision name exactly which claims passed, denied, or remain unavailable, with artifact refs and caveats?

A well-formed receipt remains evidence, not authority. Stop acceptance when the required tier has no executable support, even if cheaper tiers passed.

## Sources

- [Handbook](../README.md)
- [Distributed testing scopes and review policy](../../distributed-testing.md)
- [Governing VM fault contract](../../nixos-vm-executable-faults.md)
- [VM fault technical companion](../../technical/simulation/vm-fault-evidence-boundary.md)
- [Simulation inputs technical companion](../../technical/simulation/deterministic-simulation-inputs.md)
- [Local harness phases](../../../src/cluster_harness/parts/runner/p000/body.rs)
- [VM fault validation](../../../src/nixos/vm/parts/validation/p002/body.rs)
- [Project consensus and identity boundaries](../../../README.md)
