# World execution snapshot profiles

Aspen uses closed logical and opaque snapshot profiles. These profiles describe different restore mechanisms. They do not describe interchangeable heaps.

## Ownership

Aspen owns world meaning, logical roots, restore ordering, current admission, activation, and product receipts.

ChaosControl owns exact machine-snapshot meaning, completeness, compatibility, and machine reconstruction for an admitted opaque cohort.

VM Cohort owns the pinned copy-on-write planning contract at `31f1696ba9391bfda8577a58af84f72361d5573e`. Aspen retains workload, authority, scheduling, retention, and release decisions.

No snapshot profile owns current authority or mutable branch heads.

## Logical profile

A complete logical profile contains these Aspen-owned components:

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

A complete opaque profile contains Aspen-owned artifact, schema, runtime-profile, and policy roots. It also contains these ChaosControl-owned facts:

- exact machine descriptor;
- CPU state inventory;
- memory closure;
- device state;
- disk state;
- backend state.

Its cohort binds architecture, runtime build and ABI, KVM state profile, CPU features, vCPU topology, devices, memory format, disk format, and backend profile.

All cohort fields must match. Aspen does not fall back to logical restoration after an opaque mismatch.

Aspen pins `chaoscontrol-snapshot-descriptor` at consumer revision `b8c440ea3b19df796542e58e8ee36200e1c3db85`. The adapter validates the exact portable descriptor and maps it without importing ChaosControl policy types.

## Host handles and authority

Descriptors cannot contain file descriptors, sockets, timers, credentials, keys, transport sessions, or other live handles.

The shell obtains new handles through an explicit port. It checks current policy and authority before materialization and again before activation.

Snapshot possession does not grant capability, revocation freshness, resource admission, adapter admission, or effect permission.

## Copy-on-write clones

A clone plan binds one parent and distinct memory, device, disk, and endpoint overlays for every child.

The pure core rejects an empty child set, parent mismatch, missing overlay, oversized identity, and any overlay collision.

Aspen maps each child to one VM Cohort clone and retains both Aspen overlays and VM Cohort private-surface identities.

An explicit materialization fact supplies effective disk size. Descriptor possession cannot invent missing disk bytes.

Realization runs only through `VmCohortRealizationPort`, which is implemented by an admitted ChaosControl adapter. Aspen rejects partial activation, cleanup uncertainty, crossed plans, malformed receipts, and product-authority overclaims.

## Merge and synchronization

Logical roots can enter only their accepted root-specific merge modes.

Divergent opaque machine roots cannot enter semantic merge. Operators must select one branch, restore an ancestor, or reconcile through application logic.

The current closed profiles reject synchronization claims. Future mixed-profile work needs explicit ownership and synchronization evidence before admission.

## Operator commands

`molten world-snapshot inspect`, `compatibility`, `restore-plan`, and `clone-plan` read or write canonical bounded artifacts.

Aspen exposes exactly one executable logical profile:
`runtime-spine-committed-turn-v1`. The executable and crate names remain the
internal `molten` names. This profile resumes the existing `RuntimeState` owner:
committed assertions, observers and facets, its ordered pending-message scheduler,
logical clock, noncryptographic xorshift RNG, and effect sequence. It retains full
canonical history entries and complete unresolved clock/random request records;
restore and readback never dispatch those requests. It does **not** resume the node
daemon, native/system-extension supervisors, Wasm instances, the separate
`fabric_time::SchedulerRuntime`, live sockets, credentials or arbitrary task models.

### Capture, destination selection and materialization

An owning application constructs `LogicalRuntimeImage` from its committed
`RuntimeState::snapshot()`, retained canonical history, complete unresolved
`RuntimeEvent::EffectRequest` records and selected policy reference, then emits
`LogicalRuntimeImage::canonical_bytes()`. Capture observes the executing binary's
bytes through `/proc/self/exe` and the compiled closed ABI/schema/handler/scheduler/
time/entropy/effect profiles; it does not accept supplied cohort facts as observed
runtime facts. Capture grants no authority and does not establish source-history
correctness.

```sh
molten world-snapshot capture \
  --image image.preserves --state-root target/node \
  --trusted-signer <independently-selected-64-hex-public-key> \
  --commit-ref <world-commit-content-ref> --descriptor-out descriptor.preserves

# Provision the independently authorized current.json and signed admission.json
# described below before executing restore.
molten world-snapshot restore \
  --descriptor descriptor.preserves --destination descriptor.preserves \
  --state-root target/node \
  --trusted-signer <independently-selected-64-hex-public-key> \
  --receipt-out receipt.preserves

# This is a new process: read the persisted activation and reconstruct the owner.
molten world-snapshot readback --state-root target/node --out readback.preserves
```

`--state-root` selects an existing capability-rooted node directory with explicit
`config.preserves` and `identity.preserves` bindings. Its diagnostic path is not
canonical identity. `--destination` is only an expected-cohort assertion; restore
independently observes the current executable and rejects crossed expectations,
even when the supplied source and destination descriptors agree with each other.
Using the same descriptor above is an expectation, not compatibility authority.

Capture publishes exactly one canonical `state.preserves` leaf under each of the
eleven `<state-root>/storage/world-snapshot/bundle/<component-kind>/` directories.
Every leaf is bounded to 16 MiB and identified by BLAKE3 over its canonical packed
Preserves bytes. Extra leaves/directories, links, missing bytes, corrupt identities,
crossed semantic projections and unknown profiles deny before handle recreation.
History and unresolved requests are inline data, not unverified external content
references. The executable supplies the measured artifact implementation; schema
and runtime-profile projections must match its compiled profile. The policy
projection binds the independently admitted current policy reference.

### Current authority and live handles

The local operator authority separately provisions
`storage/world-snapshot/current.json`. Its exact `LocalCurrentAdmission` fields
are `policy_ref`, `capability_ref`, `revocation_ref`, `resource_ref`, `adapter_ref`
(canonical content references), nonzero `generation`, `allowed`, and an unexpired
`expires_unix_seconds`. These are live operator admission decisions, not facts
inferred from snapshot contents. The selected policy must equal the image policy.
The fixed image/collection bounds are part of this adapter's resource profile.

The independently authorized signer uses `sign_local_admission(LocalAdmissionInput)`
to issue `storage/world-snapshot/admission.json`: schema
`molten.runtime-spine-admission.v2`, exact descriptor/profile/cohort/destination
references, the complete current document, and Ed25519 signature over canonical
domain-separated Preserves fields. The `destination_ref` is printed by capture
and binds both node configuration and identity bytes. Provision both documents
through the operator's protected configuration channel. Never take a trust key
from the bundle or silently authorize a newly generated key. Experimental fixture
signers authorize only their disposable test roots.

The shell verifies the complete inventory and component semantics before opening
typed `DurableStoreRoot::from_ledger`/Redb staging handles. It stages
descriptor-isolated component rows, checks canonical readback, and verifies the
retained handle at the plan's handle-recreation step. `RuntimeState::from_snapshot` performs
effect-free owner reconstruction, rejecting zero RNG, invalid facet graphs,
crossed scope ownership and non-normalized value identities. Admission and node
identity/configuration are reread after reconstruction and immediately before the
activation transaction; any changed grant, revocation, generation or expiry denies
the attempt. These are fresh local observations, not a distributed revocation
oracle or an atomic fence against subsequent operator changes.
The immutable executing image and declared ABI cohort are measured once per
process; destination configuration, identity, grants and expiry remain fresh
observations on every attempt.


### Activation, failure and cleanup

The capability-derived `ledger/world-snapshot/runtime.redb` atomically records the
canonical descriptor, image and activation. A different active descriptor cannot
be replaced through this profile; the same descriptor/current grant is idempotent.
A retained activation rejects lower admission generations and changed grants at
the same generation, even if an older signed current document is reinstalled.
The library returns the actual reconstructed `RuntimeState`, not a copied file
tree or an assertion that a daemon was started. Readback reconstructs from this
typed store, verifies all projections and activation bindings against the observed
process/root, and emits the canonical image. No authority or effect dispatch is
granted by readback.

- Before commit, failures report `activation=inactive` for **this attempt**;
  descriptor-isolated partial reconstruction remains available for reconciliation.
  This does not imply that an older active world is absent.
- An activation commit error reports `activation=uncertain` and forbids automatic
  retry. Reopening may observe completed activation; a damaged/repair-requiring
  existing database denies instead of being reinitialized or silently repaired.
- Receipt-store failure after commit reports
  `activation=completed receipt-publication=failed`. Output-path failure after the
  canonical receipt is stored reports
  `activation=completed canonical-receipt=stored receipt-output=failed`.
- No failed attempt automatically deletes staged bytes, rolls back an already
  completed activation or replays unresolved effects. Cleanup is retained for
  explicit operator reconciliation, not represented as successful deletion.

Opaque machine restore and VM-cohort clone realization remain denied without
their separately admitted exact concrete adapters. Legacy file-tree bundles and
version-1 local signed grants do not acquire executable runtime semantics.

### Executable verification fixtures

`src/world_snapshot/tests/local.rs` exercises actual owner/store reconstruction,
deterministic continuation, currentness changes during reconstruction, semantic
inventory denial, untrusted/expired admission, replacement refusal and real
receipt-path failure. `tests/worldsnapshot.rs` drives separate public capture,
restore and restart/readback processes, checks canonical state and receipt bytes,
preserves unresolved requests without dispatch, and covers missing/corrupt state,
revocation, forged destination facts, unsupported profiles and opaque denial.

Parent verification commands (not an execution claim):

```sh
cargo test --lib world_snapshot
cargo test --test worldsnapshot -- --nocapture
```

These fixtures establish only the declared committed-turn profile when actually
run. They do not prove machine-loss durability, distributed activation, arbitrary
runtime/service restoration, source-history correctness or production readiness.
In particular, executing fixtures on tmpfs is not NVMe or power-loss evidence.

## Canonical records

Aspen emits canonical Preserves records for:

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
