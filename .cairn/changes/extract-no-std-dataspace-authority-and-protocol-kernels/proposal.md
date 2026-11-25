## Why

Over 30,900 LOC of completely pure deterministic runtime, authority, protocol, service-FSM, and coordination logic remain trapped inside the monolithic `std` root `molten` crate solely because they pass `preserves::IOValue` directly through their core state transitions (and are mechanically fragmented into `parts/**/body.rs` `include!()` shards):

1. **Dataspace, Vat, Turn, Envelope & Predicate Runtime (~8,340 LOC):** `src/runtime/{dataspace,vat,predicates,turn,envelope,spine}` own turn commit/rollback, assertion lifetimes, facets, causality, transactional actormaps, revocable proxies, and predicate evaluation with zero I/O (blocked only by `preserves::IOValue`, `syndicate` wrapper types, and one `std::sync` ref).
2. **Capability, Authority & Effect-Envelope Cores (~4,915 LOC):** `src/capability/`, `src/authority/`, and `src/effects/` own delegation, attenuation, claim subjects, grant currentness, revocation cleanup, and effect envelopes with zero I/O.
3. **Sans-I/O Protocol & Session Core (~3,885 LOC):** `src/protocol/{session,sans/io}` is literally named `sans_io_protocol_core`, yet resides in `std` `molten` because of `preserves::IOValue` (and `trellis`).
4. **Service Supervision, Lifecycle & Node-Profile FSMs (~7,390 LOC):** `src/service/`, `src/node/service/fsm.rs`, `src/node/profile/`, and `src/lifecycle/` own pure supervision, restart/watchdog transitions, node-profile admission, and lifecycle state machines with zero I/O.
5. **Coordination & Single-Process Control Registry State Machines (~6,390 LOC):** `src/coordination/` (~3,190 LOC) and the pure state machine inside `src/raft/control/` (~3,200 LOC mixed with Redb persistence across 21 `include!()` shards) own locks, leases, barriers, queues, elections, and service registries.

Once `molten-preserves-core` (`extract-no-std-preserves-packed-kernel`) provides `#![no_std]` canonical packed Preserves encoding, hashing, and walking, all five of these subsystems can leave the root `molten` crate as genuine `#![no_std]` + `alloc` workspace crates while retiring their `parts/**/body.rs` shards.

## What Changes

- Extract five `#![no_std]` + `extern crate alloc` workspace crates built atop `molten-preserves-core` (and `molten-runtime-admission-core`):
  - `crates/molten-dataspace-core` (dataspace turns, facets, causality, vat actormaps, predicates, envelopes, runtime spine)
  - `crates/molten-capability-authority-core` (capabilities, delegation/attenuation, authority currentness/revocation, effect envelopes)
  - `crates/molten-protocol-session-core` (sans-I/O protocol transitions and session state machines)
  - `crates/molten-service-fsm-core` (service supervision, watchdog, lifecycle, and node-profile admission FSMs)
  - `crates/molten-coordination-core` (coordination primitives + pure control-registry state transitions, separated from Redb persistence in `src/raft/control/`)
- Replace mechanical `parts/**/body.rs` `include!()` sharding across these subsystems with standard encapsulated Rust modules and keep thin `std` Preserves-text / Redb / Syndicate adapters in `src/`.
- Gate every extracted crate with `#![no_std]` compilation, unit/property tests, and workspace Octet checks in `flake.nix`.

## Impact

- **Files**: `Cargo.toml`, new workspace crates under `crates/molten-{dataspace,capability-authority,protocol-session,service-fsm,coordination}-core/`, thin adapters/re-exports in `src/runtime/`, `src/capability/`, `src/authority/`, `src/effects/`, `src/protocol/`, `src/service/`, `src/node/{service,profile}/`, `src/lifecycle/`, `src/coordination/`, `src/raft/control/`, `flake.nix`, `build-plan.json`, and `docs/architecture.md`.
- **Verification**: Hermetic `#![no_std]` compile gates for all five crates, preserved canonical packed Preserves byte identity across existing runtime/authority/protocol/coordination test suites, Clippy, workspace Octet, and strict Cairn validation.
- **Non-claims**: Moving these state machines to `#![no_std]` enforces purity and eliminates `include!()` fragmentation; semantic defect repairs remain tracked in their respective logic Cairn changes.

## Out of Scope

- Building `crates/molten-preserves-core` (prerequisite in `extract-no-std-preserves-packed-kernel`).
- Extracting governance, retention, evidence, job-DAG, and catalog Tier 2 kernels (tracked in `extract-no-std-retention-evidence-job-and-catalog-kernels`).
