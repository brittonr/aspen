Evidence-ID: alloc-safe-cluster-ticket.v1-final-validation
Task-ID: V5
Artifact-Type: command-transcript
Covers: architecture.modularity.alloc-safe-cluster-tickets-default-to-transport-neutral-bootstrap-metadata.default-and-explicit-alloc-safe-surfaces-remain-equivalent, architecture.modularity.cluster-ticket-runtime-helpers-require-explicit-shell-opt-in.cluster-ticket-seam-proof-is-reviewable, ticket.encoding.signed-cluster-ticket-decode-failures-remain-attributable-to-malformed-input.signed-cluster-ticket-encoding-proof-is-reviewable

## Review packet checklist

- `openspec/changes/alloc-safe-cluster-ticket/evidence/baseline-validation.md`
- `openspec/changes/alloc-safe-cluster-ticket/evidence/alloc-safe-validation.md`
- `openspec/changes/alloc-safe-cluster-ticket/evidence/broader-workspace-follow-up.md`
- `openspec/changes/alloc-safe-cluster-ticket/evidence/default-vs-no-default-equivalence.md`
- `openspec/changes/alloc-safe-cluster-ticket/evidence/unsigned-wire-break.md`
- `openspec/changes/alloc-safe-cluster-ticket/evidence/signed-validation.md`
- `openspec/changes/alloc-safe-cluster-ticket/evidence/direct-consumer-audit.md`
- `openspec/changes/alloc-safe-cluster-ticket/evidence/workspace-dependency-proof.txt`
- `openspec/changes/alloc-safe-cluster-ticket/evidence/implementation-diff.txt`
- `openspec/changes/alloc-safe-cluster-ticket/verification.md`

## Broader workspace fallout rails

### `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= bash -lc 'cargo check -p aspen-cli'`

warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: unknown lint: `ambient_clock`
   --> crates/aspen-cli/src/bin/aspen-cli/commands/calendar.rs:277:5
    |
277 |     ambient_clock,
    |     ^^^^^^^^^^^^^
    |
    = note: `#[warn(unknown_lints)]` on by default

warning: unknown lint: `ambient_clock`
   --> crates/aspen-cli/src/bin/aspen-cli/commands/cluster.rs:846:5
    |
846 |     ambient_clock,
    |     ^^^^^^^^^^^^^

warning: unknown lint: `ambient_clock`
   --> crates/aspen-cli/src/bin/aspen-cli/commands/cluster.rs:854:5
    |
854 |     ambient_clock,
    |     ^^^^^^^^^^^^^

warning: unknown lint: `ambient_clock`
    --> crates/aspen-cli/src/bin/aspen-cli/commands/federation.rs:1005:5
     |
1005 |     ambient_clock,
     |     ^^^^^^^^^^^^^

warning: unknown lint: `ambient_clock`
   --> crates/aspen-cli/src/bin/aspen-cli/commands/job.rs:468:5
    |
468 |     ambient_clock,
    |     ^^^^^^^^^^^^^

warning: unknown lint: `ambient_clock`
   --> crates/aspen-cli/src/bin/aspen-cli/commands/job.rs:476:5
    |
476 |     ambient_clock,
    |     ^^^^^^^^^^^^^

warning: `aspen-cli` (bin "aspen-cli") generated 6 warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.44s

### `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= bash -lc 'cargo check -p aspen-fuse'`

warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.38s

### `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= bash -lc 'cargo check -p aspen --no-default-features --features node-runtime'`

warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.40s

## Fallout note

The broader `node-runtime` rail initially failed before the rerun because `crates/aspen-cluster/src/bootstrap/node/discovery_init.rs` and `crates/aspen-cluster/src/bootstrap/node/sharding_init.rs` still returned alloc-safe `ClusterTopicId` values where runtime `iroh_gossip::proto::TopicId` was required. Both sites now call `ticket.topic_id.to_topic_id()` and the rerun passes.
