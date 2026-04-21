Evidence-ID: no-std-aspen-core.docs-review
Task-ID: 5.17
Artifact-Type: docs-review-note
Covers: core.no-std-core-baseline, architecture.modularity.feature-bundles-are-explicit-and-bounded

# `docs/no-std-core.md` review

Reviewed document: `docs/no-std-core.md`

## Checklist

- Feature matrix documents the supported shapes:
  - bare/default dependency
  - explicit `default-features = false`
  - alloc-safe `sql`
  - `std`
  - `layer`
  - `global-discovery`
  - `std,sql`
- Docs state that `default = []` and bare/default resolution is alloc-only.
- Docs call out `layer = ["std", "dep:aspen-layer"]`.
- Docs call out `global-discovery = ["std", "dep:iroh-blobs"]`.
- Docs describe that `sql` stays alloc-safe and does not require `std`.
- Docs preserve migration guidance for runtime helpers that became `std`-gated.
- Docs preserve migration guidance for `layer` and `global-discovery` users.
- Docs point reviewers at durable evidence artifacts under `openspec/changes/no-std-aspen-core/evidence/`.

## Result

Pass.

The document matches the current manifest contract in `crates/aspen-core/Cargo.toml`, the smoke-consumer proof in `crates/aspen-core-no-std-smoke/`, and the saved feature/dependency rails (`feature-claims.json`, `deps-direct.txt`, `deps-full.txt`, `deps-transitive.json`). It does not promise unsupported bare/default shell behavior and it keeps the migration guidance on the existing public paths.
