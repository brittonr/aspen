## MODIFIED Requirements

### Requirement: Acyclic no-std core boundary
ID: architecture.modularity.acyclic-no-std-core-boundary

Aspen MUST maintain an acyclic dependency boundary where the alloc-only `aspen-core` surface depends only on alloc-safe leaf crates, and `std`-bound runtime shells depend on that core rather than the reverse.

#### Scenario: Cluster types default to alloc-safe builds
ID: architecture.modularity.acyclic-no-std-core-boundary.cluster-types-default-to-alloc-safe-builds

- **GIVEN** `aspen-cluster-types` on the alloc-only foundation path
- **WHEN** the crate is built with its default production configuration or `--no-default-features`
- **THEN** it MUST compile as an alloc-safe crate without `iroh-base` in its bare/default production graph
- **AND** runtime conversion helpers MUST remain unavailable unless the consumer opts into the `iroh` feature explicitly

#### Scenario: Cluster-type verification is reviewable
ID: architecture.modularity.acyclic-no-std-core-boundary.cluster-types-verification-is-reviewable

- **GIVEN** the alloc-safe-by-default contract for `aspen-cluster-types`
- **WHEN** the seam is verified for review
- **THEN** the exact commands and results for `cargo check -p aspen-cluster-types --no-default-features`, `cargo check -p aspen-cluster-types --no-default-features --target wasm32-unknown-unknown`, `cargo test -p aspen-cluster-types`, `cargo test -p aspen-cluster-types --features iroh`, `cargo tree -p aspen-cluster-types -e normal --depth 2`, `cargo tree -p aspen-cluster-types --features iroh -e normal --depth 2`, and `cargo tree -p aspen-traits -e normal --depth 2` SHALL be saved under `openspec/changes/cluster-types-iroh-opt-in/evidence/cluster-types-validation.md`
- **AND** the saved bare/default tree artifact SHALL prove `iroh-base` is absent until `--features iroh` is enabled
- **AND** the saved alloc-safe consumer-path artifact for `aspen-traits` SHALL prove a bare/default workspace consumer path does not receive `iroh` implicitly
- **AND** the exact root-workspace stanza proof for `Cargo.toml` SHALL be saved under `openspec/changes/cluster-types-iroh-opt-in/evidence/workspace-dependency-proof.txt`
- **AND** `openspec/changes/cluster-types-iroh-opt-in/verification.md` SHALL identify which artifact proves each cluster-types claim

### Requirement: Feature bundles are explicit and bounded
ID: architecture.modularity.feature-bundles-are-explicit-and-bounded

The build feature graph SHALL distinguish alloc-only core surfaces, `std`-only shell features, and higher-level convenience bundles so enabling foundational Aspen contracts does not silently pull runtime dependencies.

#### Scenario: Cluster-type runtime helpers require per-crate opt-in
ID: architecture.modularity.feature-bundles-are-explicit-and-bounded.cluster-types-iroh-opt-in-is-explicit

- **GIVEN** a crate that directly depends on `aspen-cluster-types`
- **WHEN** it needs runtime helper APIs such as `NodeAddress::new`, `ClusterNode::with_iroh_addr`, `.iroh_addr()`, or `try_into_iroh()`
- **THEN** that crate MUST opt into `features = ["iroh"]` in its own dependency stanza
- **AND** workspace-level dependency defaults MUST NOT re-enable `iroh` implicitly for unrelated consumers
- **AND** a deterministic direct-consumer audit generated from `rg 'aspen-cluster-types\\s*=\\s*\\{' . -g 'Cargo.toml'` SHALL be saved under `openspec/changes/cluster-types-iroh-opt-in/evidence/direct-consumer-audit.md`
- **AND** that audit SHALL classify every direct consumer as either `iroh-opt-in` or `alloc-safe`, cite helper API use or non-use from `rg 'NodeAddress::new|ClusterNode::with_iroh_addr|\\.iroh_addr\\(|try_into_iroh\\(' . -g '*.rs'`, and map every consumer to saved evidence or an explicit deterministic validation rule
- **AND** every `iroh-opt-in` classification in the audit SHALL correspond to an explicit consumer manifest stanza after workspace-level feature removal rather than inherited workspace defaults
- **AND** the saved runtime/test consumer artifact SHALL include exact command/results for every direct consumer whose manifest classification changed in this seam
