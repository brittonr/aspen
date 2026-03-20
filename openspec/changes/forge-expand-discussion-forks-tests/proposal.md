## Why

The Forge has solid foundations — Git objects, issues, patches, reviews, gossip, merge, and branch protection all work. But three gaps limit it as a real collaboration platform: Discussion COBs are defined in the type system but can't be materialized, the test suite covers individual components without exercising end-to-end workflows, and there's no mechanism for forking or mirroring repositories. Closing these gaps makes the Forge usable for the self-hosted dogfood loop (Aspen building Aspen) and for multi-team collaboration across federated clusters.

## What Changes

- **Discussion COB resolver**: Add `discussion.rs` with a `Discussion` struct and `apply_change` method following the same pattern as `Issue` and `Patch`. Add `CobOperation` variants for discussion-specific operations (create thread, reply, resolve thread, lock). Wire resolution into `CobStore` with `resolve_discussion` / `list_discussions`. Add CLI commands and client RPC messages.
- **Fork & mirror support**: Add a `ForkInfo` field to `RepoIdentity` tracking upstream origin. Add `fork_repo` to `ForgeNode` that creates a new repo linked to an upstream, copies refs, and starts seeding. Add mirror mode that periodically syncs refs from an upstream (local or federated). Add CLI `repo fork` and `repo mirror` commands.
- **Integration & property tests**: Add end-to-end tests covering full workflows (create repo → push → create issue → create patch → approve → merge → gossip propagation). Add proptest generators for `CobChange` DAGs and property tests verifying COB resolution invariants (idempotency, commutativity of independent operations, convergence of concurrent changes).

## Capabilities

### New Capabilities

- `forge-discussion-cob`: Discussion COB resolution, storage, listing, and CLI commands
- `forge-fork-mirror`: Repository forking (linked clones) and mirror sync
- `forge-integration-tests`: End-to-end workflow tests and property-based COB resolution tests

### Modified Capabilities

- `forge`: New CobOperation variants for discussions, ForkInfo on RepoIdentity, new ForgeNode methods

## Impact

- **crates/aspen-forge**: New `cob/discussion.rs`, modifications to `cob/change.rs` (new CobOperation variants), `cob/store/` (discussion ops), `identity/mod.rs` (ForkInfo), `node.rs` (fork/mirror methods), new test files
- **crates/aspen-client-api**: New RPC request/response types for discussions, fork, mirror
- **crates/aspen-rpc-handlers**: New handler methods for discussion/fork/mirror RPCs
- **crates/aspen-cli**: New `discussion` subcommand, new `repo fork` / `repo mirror` commands
- **crates/aspen-forge/tests/**: New integration test files
- **openspec/specs/forge/spec.md**: Updated with discussion, fork, mirror requirements
