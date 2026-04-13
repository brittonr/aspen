## Phase 1: Baseline and acceptance rails

- [x] Save a crate-graph and hotspot baseline for `aspen-client-api`, `aspen-rpc-core`, `aspen-rpc-handlers`, `aspen-cluster`, `src/bin/aspen_node/setup/client.rs`, and `src/node/mod.rs`, including direct evidence for the current god-context, mega-enum, feature-knot, and `unsafe` transmute seams.
- [x] Add acceptance checks that future slices must keep green: client wire-format golden tests, representative feature-slice cargo checks, handler-registration tests, and bootstrap compile/start smoke tests.

## Phase 2: Shared-type and feature-bundle cuts

- [x] Move the shared Raft / transport type configuration behind one leaf crate boundary and remove the `transmute_raft_for_transport` `unsafe` bridge from `src/node/mod.rs`.
- [x] Split leaf features from convenience bundles in `crates/aspen-cluster/Cargo.toml`, `crates/aspen-rpc-core/Cargo.toml`, `crates/aspen-rpc-handlers/Cargo.toml`, and root feature wiring so direct prerequisites stay explicit and bundle names document cross-app composition.

## Phase 3: Handler capability boundaries

- [x] Refactor `ClientProtocolContext` in `crates/aspen-rpc-core/src/context.rs` into declared capability traits or bounded subcontexts, and update test builders to construct only the capabilities under test.
- [x] Move handler linking / registration ownership outward so `aspen-rpc-core` remains trait-focused and `aspen-rpc-handlers` or the binary layer assembles concrete domain handlers without reintroducing a global dependency bag.

## Phase 4: RPC ownership and bootstrap phases

- [x] Reduce the central ownership burden in `crates/aspen-client-api/src/messages/mod.rs` by moving request metadata and domain ownership toward domain-scoped modules while preserving golden-wire compatibility.
- [x] Split `src/bin/aspen_node/setup/client.rs`, `crates/aspen-cluster/src/bootstrap/`, and `src/node/mod.rs` into explicit startup phases and per-domain installers so minimal node compositions can build without the current large feature conjunctions.

## Phase 5: Verification and cleanup

- [x] Save post-change evidence showing reduced central edit hotspots, removed `unsafe` type bridging, explicit feature bundles, and bounded bootstrap phases.
- [x] Run `scripts/openspec-preflight.sh decouple-cross-crate-boundaries` and add `verification.md` plus durable evidence before any task is checked complete.
