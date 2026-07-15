## Context

Git dependencies can be reproducible only when the reviewed manifest, Nix source inputs, and Cargo lockfile identify immutable source revisions. A lockfile-resolved commit does not make a floating manifest declaration reviewable, and a manifest pin does not help if the lockfile resolves another source. Canonical Valence ownership also requires Aspen to stop selecting Octet's hosted package implementation.

## Decisions

### Require immutable release source coordinates

Every Git dependency in the release closure records an immutable revision in the repository-owned manifest or generated source-of-truth export. Branches and tags may remain display metadata but cannot be the binding release identity. SSH transport is permitted only when the immutable revision and fetch policy remain reproducible in the Nix build path.

A pure dependency check compares normalized manifest, lockfile, and Nix source rows. Shell code loads those files; the core reports floating source, missing source, revision mismatch, duplicate package identity, and unsupported transport policy deterministically.

### Consume canonical standalone Valence

Aspen replaces the Octet-hosted `valence-core` source with the exact standalone Valence revision accepted by Octet's completed cutover. Dependency validation rejects the Octet-hosted semantic package and rejects multiple different source identities under canonical `valence-core` package identity.

### Keep AGPL as an allowed distribution profile

The project may distribute Aspen under AGPL. A typed release-distribution profile records the selected license identity, notice artifacts, source coordinate, source revision, and project-required corresponding-source/export evidence. Missing evidence blocks the configured distribution profile, but the AGPL choice itself does not.

This policy evidence is not legal advice and does not claim universal legal compliance.

### Cross-repository ordering

Implementation requires archived receipts for Valence `harden-preserves-integrity-boundaries` and Octet `complete-standalone-valence-cutover`. The exact accepted source identities from those receipts become Aspen dependency-policy inputs. These prerequisites remain documented until Cairn cross-repository dependency metadata is available.

### CI scope

The checked-in workflow runs only `nix flake check`. Focused dependency, lock-drift, and distribution-profile tests remain required implementation evidence outside the minimal CI command scope.

## Risks / Trade-offs

- Exact pins require deliberate update work, but prevent branch movement from changing release inputs silently.
- SSH sources may be unavailable in sandboxed or public CI; the Nix source path must define a reproducible fetch mechanism or reject the dependency.
- Distribution evidence cannot settle legal interpretation; it records project policy and source availability facts only.
