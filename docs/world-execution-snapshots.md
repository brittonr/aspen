# World execution snapshot profiles

Molten uses closed logical and opaque snapshot profiles. These profiles describe different restore mechanisms. They do not describe interchangeable heaps.

## Ownership

Molten owns world meaning, logical roots, restore ordering, current admission, activation, and product receipts.

ChaosControl owns exact machine-snapshot meaning, completeness, compatibility, and machine reconstruction for an admitted opaque cohort.

VM Cohort owns the pinned copy-on-write planning contract at `31f1696ba9391bfda8577a58af84f72361d5573e`. Molten retains workload, authority, scheduling, retention, and release decisions.

No snapshot profile owns current authority or mutable branch heads.

## Logical profile

A complete logical profile contains these Molten-owned components:

1. artifact root;
2. schema root;
3. durable-state root;
4. task root;
5. history root;
6. effect-state root;
7. scheduler root;
8. virtual-time root;
9. entropy root;
10. runtime-profile root;
11. policy root.

Its cohort binds the runtime build and ABI, schema set, handler set, task model, scheduler, time, entropy, and effect profiles.

The shell validates every component before state restoration. It restores state in a fixed order, recreates host handles, rechecks current admission, and activates last.

A missing or unverified component stops restoration. A stale admission after restoration prevents activation and success-receipt publication.

## Opaque profile

A complete opaque profile contains Molten-owned artifact, schema, runtime-profile, and policy roots. It also contains these ChaosControl-owned facts:

- exact machine descriptor;
- CPU state inventory;
- memory closure;
- device state;
- disk state;
- backend state.

Its cohort binds architecture, runtime build and ABI, KVM state profile, CPU features, vCPU topology, devices, memory format, disk format, and backend profile.

All cohort fields must match. Molten does not fall back to logical restoration after an opaque mismatch.

Molten pins `chaoscontrol-snapshot-descriptor` at consumer revision `b8c440ea3b19df796542e58e8ee36200e1c3db85`. The adapter validates the exact portable descriptor and maps it without importing ChaosControl policy types.

## Host handles and authority

Descriptors cannot contain file descriptors, sockets, timers, credentials, keys, transport sessions, or other live handles.

The shell obtains new handles through an explicit port. It checks current policy and authority before materialization and again before activation.

Snapshot possession does not grant capability, revocation freshness, resource admission, adapter admission, or effect permission.

## Copy-on-write clones

A clone plan binds one parent and distinct memory, device, disk, and endpoint overlays for every child.

The pure core rejects an empty child set, parent mismatch, missing overlay, oversized identity, and any overlay collision.

Molten maps each child to one VM Cohort clone and retains both Molten overlays and VM Cohort private-surface identities.

An explicit materialization fact supplies effective disk size. Descriptor possession cannot invent missing disk bytes.

Realization runs only through `VmCohortRealizationPort`, which is implemented by an admitted ChaosControl adapter. Molten rejects partial activation, cleanup uncertainty, crossed plans, malformed receipts, and product-authority overclaims.

## Merge and synchronization

Logical roots can enter only their accepted root-specific merge modes.

Divergent opaque machine roots cannot enter semantic merge. Operators must select one branch, restore an ancestor, or reconcile through application logic.

The current closed profiles reject synchronization claims. Future mixed-profile work needs explicit ownership and synchronization evidence before admission.

## Operator commands

`molten world-snapshot inspect`, `compatibility`, `restore-plan`, and `clone-plan` read or write canonical bounded artifacts.

`molten world-snapshot restore` has no ambient runtime adapter. It writes a canonical denial receipt and fails until an admitted shell supplies the restore capability.

## Canonical records

Molten emits canonical Preserves records for:

- snapshot descriptors;
- completeness inventories;
- compatibility reports;
- restore plans;
- clone plans;
- bounded receipts.

Each record class has a separate domain-separated BLAKE3 identity. Descriptor decoding rejects unknown schemas, profiles, components, cohort facts, owners, malformed references, unsafe handles, and non-normalized order.

## Non-claims

Snapshot records do not prove:

- guest or workload correctness;
- cross-host or future portability;
- current authority;
- host-handle transfer;
- logical and opaque semantic equivalence;
- clone realization correctness or isolation beyond recorded observations;
- release eligibility.

Cargo, Cargo lock, Nix, Nix lock, the release profile, and generated plans bind both reviewed revisions. Focused tests preserve all authority and portability non-claims.
