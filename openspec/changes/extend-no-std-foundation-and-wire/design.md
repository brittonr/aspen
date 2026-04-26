## Context

`aspen-core` (alloc-only) and `aspen-core-shell` (std wrapper) form a
two-tier API surface. Shell consumers alias `aspen-core-shell` as
`aspen-core` via Cargo dependency key. The UI fixture tests validate that
specific types are importable under each tier's constraints.

## Goals / Non-Goals

**Goals:** Make all 8 UI fixtures pass.
**Non-Goals:** Full no-std support for runtime/async types. Moving iroh
or redb dependencies into the alloc-only tier.

## Decisions

### 1. Feature-gate relaxation (fixtures 5-6)

**Choice:** Remove the `global-discovery` gate from `ContentDiscovery`
trait and the `layer` gate from `DirectoryLayer` struct in
`aspen-core-shell`.

**Rationale:** The trait/struct definitions don't inherently need the
feature — only their runtime implementations and backing crates do. The
types can be exported unconditionally while keeping implementation code
gated.

**Alternative:** Create stub types. Rejected because the real types are
already available and only hidden by overly broad gates.

### 2. Alloc-safe type subsets (fixtures 1-4)

**Choice:** For each std-dependent type, extract an alloc-safe data model
into `aspen-core` and keep the std-only persistence/runtime layer in
`aspen-core-shell`.

**Rationale:** The UI fixtures only need the type to exist in no-std mode
(they test `size_of` or similar). Splitting data from behavior lets both
tiers work.

**Risk:** Types with deep std dependencies (`SimulationArtifact` uses
`PathBuf`, `chrono`, `anyhow`; `AppRegistry` uses `HashMap`, `tokio`)
require significant refactoring to create alloc-safe subsets.

### 3. `SM_KV_TABLE` (fixture 4)

**Choice:** Move the redb `TableDefinition` into a separate
`aspen-storage-redb` or keep it in `aspen-storage-types` behind a feature
gate, then provide a no-std-safe type alias or name constant in
`aspen-core`.

**Rationale:** `SM_KV_TABLE` is a `redb::TableDefinition` which
requires std. The no-std fixture can only access a table name string
or an abstract definition.

## Risks / Trade-offs

**[Scope creep]** Each type extraction may cascade into dependency graph
changes. Mitigate by doing one type at a time as independent sub-changes.

**[API divergence]** Alloc-safe subsets may not have all fields of the
std types. Mitigate by using the same struct name with cfg-conditional
fields, or separate types with `From` conversions.
