# Redb Raft KV manifest index

This artifact anchors checked task `I2` after the initial manifest files were committed. It lists the canonical first-slice Redb Raft KV extraction manifests and the policy fields they contain.

## Canonical manifests

- `docs/crate-extraction/aspen-kv-types.md`
- `docs/crate-extraction/aspen-raft-kv-types.md`
- `docs/crate-extraction/aspen-redb-storage.md`
- `docs/crate-extraction/aspen-raft-kv.md`
- `docs/crate-extraction/aspen-raft-network.md`
- `docs/crate-extraction/aspen-raft-compat.md`

## Required policy coverage

Each manifest records candidate name/family, intended audience, crate/category class, documentation entrypoint, package description, license policy, repository/homepage policy, default and optional feature sets, public API owner, semver/compatibility policy, internal and external dependencies, binary/runtime dependency stance, keep/move/feature-gate/remove decisions, release-readiness state, dependency-policy class, compatibility or canonical path, representative consumers/re-exporters, compatibility re-export plan, dependency exceptions, and mandatory first-slice verification rails.
