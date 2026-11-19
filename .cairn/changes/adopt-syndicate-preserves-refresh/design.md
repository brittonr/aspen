## Context

The audit's advisory set is dominated by transitive parents. One of them,
`syndicate`, already ships the fix in a later release candidate, so the only
blocker is the `preserves` version that candidate requires — which our own
Choregraph pin fixes at `=5.0.0-rc.7`.

## Decisions

### 1. Move Choregraph first, then Molten

**Choice:** bump `preserves` in Choregraph, regenerate the two Kamacite receipts
its corpus test pins, publish `318d4932`, then repin Molten.

**Rationale:** Cargo resolves `preserves 5.x` to one version in the graph, so the
consumer cannot move before the source that pins it. Choregraph's corpus test
exists precisely to force receipt regeneration when the encoder changes; 291
tests, clippy and rustfmt pass after regeneration.

### 2. Repin the whole revision surface in one change

**Choice:** update both Molten manifests, the flake input and lock, the release
profile, the four focused Octet workspaces and their locks, the plans, the hashes,
the third-party notice and the four documents that quote the revision.

**Rationale:** the audit's release guard fails closed on any partial repin — it
reported `DuplicatePackageIdentity` while `molten-core` still pinned the old
revision, and `NixDrift` while the flake input did not match the profile. A pin
is one identity across manifests, Nix, profile, notices and docs, so a partial
change is indistinguishable from drift.

### 3. No lock regeneration beyond the pin

**Choice:** let Cargo re-resolve with `cargo metadata` and inspect the result;
do not run a fresh `generate-lockfile`.

**Rationale:** a fresh resolve drifted 262 package versions in this workspace
(`base64 0.22 -> 0.23`, `hickory -> n0-dns-resolver`, ...), which would silently
replace the pinned identities the audit exists to protect. The selected lock
changes only the choregraph, syndicate and preserves edges.

## Risks / Trade-offs

- `syndicate 0.44.0-rc.9` is a release candidate; the flow-control harness that
  uses it has no behaviour change and the workspace checks pass.
- `preserves 5.0.0-rc.10` changes serialization only for the two regenerated
  receipts; the audit's own report bytes are rebound in the evidence.
