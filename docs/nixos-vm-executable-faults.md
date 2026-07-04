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

`molten test nixos-vm fault-validate` parses descriptors and receipts against the topology. It denies log-only pass claims, wrong topology, missing injection or child refs, unavailable pass claims, unsupported host pass claims, and missing diagnostic evidence for denial receipts.

## Host support and unavailable handling

Host support is explicit: `supported`, `unavailable`, or `denied`. Missing KVM, QEMU, test-driver, network-control, filesystem, or privilege support must produce `unavailable` or `deny` evidence. Unsupported execution must never be converted into pass evidence.

## Fault classes

Supported descriptor kinds include network delay/drop/partition/rejoin, asymmetric latency, crash/restart, duplicate send after restart, receipt write/readback, missing artifact, permission-denied state root, bounded disk pressure, unsupported host feature, tampered fault receipt, wrong topology, and log-only pass negative fixtures.

## Operator inspection

Inspect the realized VM check output for descriptor, receipt, validation, topology, node evidence, and test-run artifacts. Logs are diagnostic-only and cannot override canonical deny receipts.
