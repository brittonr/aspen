## Context

This change turns a modularity audit into an executable Tiger Style refactor plan.

Current hotspots:

1. `ClientRpcRequest` / `ClientRpcResponse` concentrate cross-domain protocol edits in one file.
2. `ClientProtocolContext` acts as a global dependency bag.
3. node setup composes many apps in `setup/client.rs` instead of per-domain installers.
4. bootstrap and `NodeBuilder::start()` require large feature conjunctions.
5. Raft and transport still share type shape through `unsafe` transmute.
6. feature bundles mix direct prerequisites with convenience bundles.

Tiger Style direction is narrow interfaces, deterministic core, explicit ownership, bounded orchestration phases, and loud composition edges.

## Goals / Non-Goals

**Goals**

- Reduce central edit hotspots for new domain work.
- Replace global dependency bags with declared capability sets.
- Split node composition into bounded install phases.
- Remove `unsafe` type bridging where a leaf crate can hold the shared model.
- Make feature bundles explicit, documented, and independently testable.
- Keep migration incremental so wire compatibility and node behavior stay stable.

**Non-Goals**

- Rewrite all RPC payload types in one step.
- Change Aspen's external transport model away from Iroh.
- Replace Raft or the current handler registry.
- Land every refactor in one PR.
- Guarantee zero compile-time coupling across the whole workspace.

## Decisions

### 1. Define modularity through bounded seams, not one large rewrite

**Choice:** break the work into five seams: RPC contracts, handler context, bootstrap composition, shared protocol types, and feature bundles.

**Rationale:** one giant rewrite would destabilize the control plane. Five seams let the repo improve one boundary at a time with direct acceptance tests.

### 2. Keep external wire behavior stable while moving ownership inward

**Choice:** first move request metadata and app ownership out of central files while preserving the outer client envelope and golden tests.

**Rationale:** Tiger Style prefers safe, incremental control flow. Internal ownership can move before any external protocol migration.

### 3. Replace god-context access with declared capabilities

**Choice:** handlers and installers should declare what they require, and composition code should provide only those capabilities.

**Rationale:** explicit dependencies reduce accidental coupling and make negative space testable: a handler without a capability should fail registration clearly instead of carrying unused fields forever.

### 4. Compose startup in phases with explicit outputs

**Choice:** bootstrap should produce bounded phase outputs such as storage, transport, rpc, and app services. Later phases consume typed outputs from earlier phases.

**Rationale:** this follows functional core / imperative shell structure. Phase planning stays explicit, side effects stay local, and minimal node slices become buildable.

### 5. Move shared consensus / transport types to one leaf source

**Choice:** any type shared by `aspen-raft` and `aspen-transport` should live in a leaf crate that neither side must transmute around.

**Rationale:** `unsafe` here is architectural debt, not algorithmic necessity. One shared leaf crate is simpler, safer, and easier to audit.

### 6. Distinguish leaf features from convenience bundles

**Choice:** leaf features enable only direct prerequisites. Larger cross-app compositions become explicitly named bundles.

**Rationale:** `ci`, `hooks`, and bootstrap currently hide unrelated pulls. Explicit bundles make compile slices predictable and bounded.

## Architecture

### A. RPC contracts

Target shape:

- one stable outer client envelope
- per-domain request / response modules own their payloads and metadata
- central dispatch knows domain IDs and registration, not every payload variant detail
- golden tests pin wire behavior during migration

### B. Handler capabilities

Target shape:

- small capability traits or subcontexts (`KvCapability`, `ForgeCapability`, `CiCapability`, `MetricsCapability`)
- handler factory declares required capabilities
- binary composition fails fast when capability contract is incomplete
- test builders construct only the capabilities under test

### C. Bootstrap phases

Target shape:

- `phase_storage`
- `phase_transport`
- `phase_rpc`
- `phase_apps`
- optional app installers register themselves into the RPC / bootstrap shell

Each phase returns typed outputs. No later phase reaches back into raw globals.

### D. Shared leaf types

Target shape:

- move shared Raft / transport config types into `aspen-raft-types` or another leaf crate
- both crates import shared definitions directly
- remove `unsafe` transmute and keep static assertions where layout still matters elsewhere

### E. Feature bundles

Target shape:

- leaf feature: direct code and direct prerequisites only
- bundle feature: named aggregate with clear purpose
- compile-slice tests cover representative minimal and full bundles

## Risks / Trade-offs

**Incremental migration keeps temporary adapters alive** -> short-term duplication may increase.
**Mitigation:** document migration order and delete adapters as soon as the next seam lands.

**Wire compatibility regressions** -> moving metadata ownership can shift discriminants or routing.
**Mitigation:** keep postcard golden tests and explicit no-drift checks for every migration step.

**Bootstrap fragmentation** -> phase extraction can scatter control flow.
**Mitigation:** keep one top-level orchestrator that calls small phase functions in fixed order.

**Feature churn** -> renaming bundles can break local workflows.
**Mitigation:** keep compatibility aliases during migration and verify documented build commands still work.

## Verification Strategy

- save baseline crate-graph evidence for current coupling hotspots
- add compile-slice checks for minimal and bundled feature sets
- keep wire-format golden tests green during RPC ownership changes
- add handler registration tests that assert missing capabilities reject cleanly
- add bootstrap phase tests that prove minimal node compositions compile and start
- record removal of the `unsafe` transmute with targeted source diff evidence
