# VM fault evidence boundary

A VM test can observe real platform behavior while still supporting only a narrowly bounded claim. Molten preserves this distinction through fault descriptors, receipts, topology binding, host-support status, and evidence scopes. This article assumes the [NixOS VM executable fault guide](../../nixos-vm-executable-faults.md) and the [distributed testing evidence guide](../../distributed-testing.md). The [Technical companion](../README.md) provides navigation to related implementation analyses.

## Three different questions

Reviewing a fault result requires separating three questions. Was the intended experiment described completely? Did the supported environment actually execute the intervention and child workflow? Does the resulting evidence justify the particular requested claim? A synthetic descriptor can answer the first question without answering the second. A successful service restart can answer part of the second without granting authority, policy, deployment trust, or broad transport correctness.

The executable-fault guide defines `nixos-vm-fault-descriptor-v1` as a binding of topology, target node and optional link, fault kind, command profile, expected outcome, bounded duration or trigger, preflight refs, and caveats. The corresponding receipt binds the descriptor to host support, pre-fault observations, injection refs, child workflow refs, post-fault refs, replay status, diagnostics, logs, and caveats. The descriptor expresses intent; the receipt supplies the execution-facing evidence relationships.

This is a stronger review surface than terminal output, but not a reason to ignore provenance. A correctly shaped reference is not itself proof that a live child operation happened. The surrounding topology, node evidence, shard scope, and execution artifacts remain necessary to interpret the reference.

## What the validation code checks

The inspected [fault-validation implementation](../../../src/nixos/vm/parts/validation/p002/body.rs) rejects missing descriptor and receipt collections. For each descriptor it checks identity presence, topology-ref equality, target-node membership, minimum bounded duration, and caveats. For each receipt it checks that the descriptor reference is present in the supplied descriptor set.

A pass receipt requires `supported` host status plus nonempty pre-fault, injection, child, and post-fault reference collections. An empty child collection is explicitly diagnosed as a log-only pass. Non-pass receipts need diagnostic evidence. Every receipt also needs log refs, replay status, and caveats. An expected unavailable outcome cannot be reported as pass, and the log-only-pass negative fault kind cannot be promoted to pass.

These checks establish rejection conditions over parsed evidence. They should not be paraphrased as a general theorem that every injection ref denotes a physically successful injection. Review the command profile and artifact-producing execution in addition to the validator. Likewise, “logs are diagnostic-only” does not mean logs are useless or always optional: the inspected validator requires log refs while refusing to let them replace child evidence.

## Host support is an input to interpretation

Host support is explicit: supported, unavailable, or denied. Missing KVM, QEMU, test-driver, network-control, filesystem, or privilege support is not an alternative successful implementation. The [distributed testing guide](../../distributed-testing.md) requires network-control probing before executable delay, drop, partition, rejoin, or asymmetric-latency claims. An unsupported run can provide useful diagnostic evidence about its environment without satisfying a platform-pass claim.

The same guide distinguishes shard scopes: fixture metadata for synthetic planning/read-back, executable VM for real VM child receipts, aggregate index for index-only artifacts, and diagnostic-only for unavailable or log evidence. An aggregate preserves these child scopes rather than laundering them into a stronger result. A parent record that mentions a child cannot increase what the child actually observed.

## Worked unavailable-partition example

Suppose, illustratively, a two-node scenario intends to partition the link to `node-b`, then observe a bounded child workflow and restore connectivity. Its preflight finds network control unavailable. A useful report can still bind the requested descriptor, unavailable status, diagnostics, and log refs. It cannot truthfully claim that the partition was injected and tolerated.

If a wrapper changes that receipt's decision to pass while leaving unavailable host support, validation emits the unavailable-cannot-pass diagnostic. If it also omits child refs and supplies only a driver log saying “scenario finished,” the log-only-pass condition applies independently. Adding unrelated child refs would not be a sound review repair: evidence must correspond to the declared experiment, and the unavailable boundary remains unresolved.

Contrast a genuinely executed live exchange. The governing guide requires the live-transport gate to bind sender, receiver, expected peer, topic, operation identity, ticket, admission, authority, send, receive, ingress, queue, dispatch, reconciliation, acknowledgement, and protocol-gate refs. Copying artifacts from one VM to another after the test is export plumbing, not the receive operation. This distinction prevents test-driver convenience from masquerading as transport execution.

## Replay and verification guidance

Deterministic simulation may reproduce its scheduler trace. VM and local multiprocess failure bundles instead verify as non-replayable diagnostic evidence unless a separate recorded effect log exists, according to the distributed testing guide. A replay-status field should therefore be interpreted in its profile, not used to import simulation guarantees into a QEMU run.

Suggested verification is to inspect realized VM output for descriptors, validation, support matrix, topology, node evidence, child receipts, and logs, then trace each pass claim to executable child scope. The documented `nix build .#checks.x86_64-linux.nixos-vm-multinode` command is an appropriate platform entry point when host support exists; it was not run for this article. Negative review should cover unavailable support, wrong topology, missing injection refs, and log-only success.

These artifacts provide bounded platform-integration evidence. They do not establish WAN behavior, fleet scale, production readiness, universal fault tolerance, or authority beyond the tested workflow. The [whole-system simulation guide](../../fabric-whole-system-simulation.md) remains a separate, weaker-in-environment but deterministic evidence profile, not an interchangeable substitute.

## Sources

- [NixOS VM executable fault evidence](../../nixos-vm-executable-faults.md)
- [Distributed testing evidence and scopes](../../distributed-testing.md)
- [Whole-system simulation claim boundary](../../fabric-whole-system-simulation.md)
- [Fault evidence validation implementation](../../../src/nixos/vm/parts/validation/p002/body.rs)
- [VM receipt and shard construction](../../../src/nixos/parts/vm/p001/body.rs)
- [VM fault validation fixtures](../../../src/nixos/vm/parts/validation/tests/m000/p001/body.rs)
- [Technical companion](../README.md)
