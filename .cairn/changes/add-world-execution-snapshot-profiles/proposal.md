## Why

The world-commit core can reference logical runtime roots and optional opaque machine snapshots. It does not define complete capture, compatibility, restore, or copy-on-write rules for those execution-state classes.

Molten logical continuations and ChaosControl machine snapshots preserve different facts. VM Cohort also plans reusable checkpoint cloning. One untyped memory root would erase these boundaries and invite unsafe restore or merge claims.

## What Changes

- Add closed logical and opaque execution-snapshot profiles for Molten world commits.
- Bind each profile to exact runtime, architecture, schema, topology, device, storage, time, entropy, and adapter cohort identities.
- Define completeness inventories and fail closed when any profile-required state is absent.
- Map logical profiles to Molten task, scheduler, durable-state, time, entropy, and effect roots.
- Map opaque profiles to reviewed ChaosControl snapshot manifests and exact compatibility checks.
- Consume VM Cohort checkpoint-clone mechanics after its implementation passes the ChaosControl pilot.
- Bind copy-on-write children to one exact parent snapshot and isolated memory, device, disk, and endpoint overlays.
- Recreate host handles and recheck current authority during restore. Do not serialize live handles or capabilities.

## Dependencies

- `introduce-world-commit-core`.
- ChaosControl `exact-x86-kvm-v1` snapshot fidelity and machine-readable replay evidence.
- VM Cohort `build-vm-cohort` before copy-on-write cohort activation.
- Existing Molten logical task, scheduler, virtual-time, entropy, effect, runtime-profile, and VM evidence contracts.

## Non-Goals

- Semantic merge of opaque memory, CPU, devices, disks, or live process heaps.
- Cross-architecture restore, silent compatibility fallback, or universal VM migration.
- Treating snapshot identity as current authority, host confinement, or application correctness.

## Impact

- **Core**: profile DTOs, cohort identities, completeness inventories, compatibility, restore plans, clone plans, and diagnostics.
- **Shell**: Molten logical adapters, ChaosControl snapshot adapters, VM Cohort clone adapters, host-handle recreation, and authority rechecks.
- **Schemas**: execution-snapshot descriptors, inventories, compatibility reports, and restore receipts.
- **Testing**: successful logical and opaque restore plus negative incomplete state, cohort mismatch, live-handle capture, unsafe merge, overlay collision, and authority-staleness cases.
