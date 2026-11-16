# NixOS VM executable fault evidence

Executable VM fault checks exercise bounded platform faults inside the NixOS test topology when the host and VM image support them. The fault layer is platform-integration evidence only; it does not grant authority, policy, provenance, resource, source-gate, retention, destructive-operation, deployment, or transport trust beyond the tested topology.

## Canonical artifacts

`molten test nixos-vm fault-descriptor` creates a `nixos-vm-fault-descriptor-v1` artifact that binds:

- topology ref;
- target node and optional link;
- fault kind;
- command profile;
- expected outcome;
- bounded duration or trigger;
- preflight evidence refs;
- evidence-only caveats.

`molten test nixos-vm fault-receipt` creates a `nixos-vm-fault-receipt-v1` artifact that binds the descriptor ref, host-support status, pre-fault refs, injection refs, child workflow refs, post-fault refs, replay status, diagnostics, diagnostic log refs, and caveats.

`molten test nixos-vm fault-validate` parses descriptors and receipts against the topology. It denies log-only pass claims, wrong topology, missing injection or child refs, unavailable pass claims, unsupported host pass claims, and missing diagnostic evidence for denial receipts. For `denial-observed-before-mutation`, it additionally requires actual `--attempt-artifact`, `--pre-observation` and `--post-observation` values. Their canonical hashes must match the receipt's child/pre/injection/post refs; the validator checks the operation, selected package/executable, startup binding/config, effective identity and DAC permissions, typed EACCES at `open-before-write`, absent target and identical protected-tree identities. Nonempty unrelated refs, a handwritten marker or a log cannot qualify this claim. An unavailable receipt remains valid structural reporting, not execution evidence.

The pure support-matrix model emits `nixos-vm-fault-support-matrix-v1` with each fault kind, required capability, target, command profile, expected outcome, host-support status, preflight refs, injection refs, child workflow refs, post-fault refs, diagnostics, and caveats. Reviewers should read this compact table before treating a VM fault check as executable evidence.

## Host support and unavailable handling

Host support is explicit: `supported`, `unavailable`, or `denied`. Missing KVM, QEMU, test-driver, network-control, filesystem, or privilege support must produce `unavailable` or `deny` evidence. Unsupported execution must never be converted into pass evidence.

## Fault classes

Supported descriptor kinds include network delay/drop/partition/rejoin, asymmetric latency, crash/restart, duplicate send after restart, receipt write/readback, missing artifact, permission-denied state root, bounded disk pressure, unsupported host feature, tampered fault receipt, wrong topology, and log-only pass negative fixtures.

The signed VM rail now attempts the **existing** `molten node status` health-file write against node_b's protected state root, after stopping the service and making the root unwritable to its non-root service identity. The CLI emits `node-state-operation-attempt-v1` outside the root only when the actual `health-receipt.preserves` open fails with typed EACCES before a handle exists; normal status still fails. Separate root-observer processes emit `nixos-vm-state-observation-v1` before and after, binding the bounded protected tree, absent target, root mode/owner, selected package, executable, startup binding and config. The attempt records actual UID, GID, groups and effective capabilities from `/proc/self/status` and the executing ELF ref. A post-open write/flush error or a capability-admission denial is not this scenario. Missing service identity, denial artifact or independent observation emits `unavailable` with a reason; source edits alone are not an observed VM result.

The observer opens the selected state and package directory capabilities once and reads members relative to them without following leaf symlinks. It rejects changed file metadata, unsupported entries, more than 4,096 tree entries or more than 256 MiB of protected content. Process-status capture is limited to 65,536 bytes and 64 supplementary groups; executable hashing is streamed under the node-host executable bound. Observation and attempt outputs use capability-relative no-replace publication outside the protected root.

The opt-in `nix run .#nixos-vm-multinode-signed` rail uses the signed source-binding output's `bin/molten` for the VM service and test-driver operations and binds that exact output in the topology. The fixture's VM-local signing proves the mechanism only. The separate `nix build .#checks.x86_64-linux.nixos-vm-multinode` checks missing-trust denial, not positive signed startup; neither source changes nor a structural receipt establish that a VM run has completed.

The signed campaign declares 8 vCPUs and 8 GiB per VM and a four-hour aggregate driver budget for its multiple source-admission/scenario phases. The complete closure is copied into a local read-only Nix store image instead of using host-store 9p; source verification is unchanged. Individual startup waits and the service's startup deadline remain 3,600 seconds; no admission predicate or operation deadline is relaxed. The earlier [one-hour-driver attempt](../.cairn/changes/observe-real-vm-storage-denial/evidence/signed-vm-timeout-2026-11-16.json) exhausted the aggregate budget, and the later [8-vCPU/8-GiB attempt](../.cairn/changes/observe-real-vm-storage-denial/evidence/signed-vm-eight-cpu-timeout-2026-11-16.json) exhausted the individual startup wait. Neither reached a storage-denial result. The store-image profile still requires actual execution; a profile edit is not a passing rerun or a production startup-latency claim.

## Operator inspection

Inspect the realized VM check output for descriptor, receipt, validation, support-matrix, topology, node evidence, and test-run artifacts. Logs are diagnostic-only and cannot override canonical deny receipts.

For true cross-node live transport, inspect `nixos-vm-live-transport-gate-v1`: it must bind sender, receiver, expected peer, topic, operation id, ticket, peer admission, authority, send, receive, ingress, queue, dispatch, reconcile, ack, and protocol-gate refs. Test-driver artifact copying is review/export plumbing only after the live exchange completes; it cannot replace the receive receipt.
