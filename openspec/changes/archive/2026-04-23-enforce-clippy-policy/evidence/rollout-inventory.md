# Enforced Clippy rollout inventory

## Canonical rollout scope

This change enforces the deny baseline for these crate roots:

- `crates/aspen-time/src/lib.rs` (`aspen-time`)

## Why this initial scope

- `aspen-time` already passed `cargo clippy -D warnings`, has complete crate docs, and carries existing checked-in Tiger Style crate-level policy that can be composed with the standardized Clippy deny block without a large cleanup blast radius.

## Deferred crates

All other workspace crates are deferred from this first enforcement pass. The baseline workspace `cargo clippy --workspace --all-features --all-targets -D warnings` currently fails before a whole-repo deny flip is reviewable.

| Crate | Deferred reason | Owner / follow-up |
| --- | --- | --- |
| `aspen-core-shell` | already carries broad crate-root Tiger Style allow-list and needs a dedicated cleanup to avoid layering a second broad escape hatch while preserving current policy | follow-up change required: `tighten-core-shell-lint-suppressions` |
| `aspen-rpc-handlers` | crate root currently carries broad crate-level Clippy allowances (`collapsible_if`, `redundant_closure`, `iter_cloned_collect`, `too_many_arguments`) that must be narrowed before deny rollout | follow-up change required: `narrow-rpc-handlers-clippy-allows` |
| `aspen-federation` | crate root currently carries broad crate-level allowances and needs item-scoped cleanup first | follow-up change required: `narrow-federation-clippy-allows` |
| all remaining workspace crates | not yet inventoried for pedantic + missing docs cleanup in this change; keeping them explicit workspace backlog avoids claiming repo-wide enforcement without proof | owner: Aspen maintainers, tracked by this change as deferred rollout backlog |

## Baseline command for explicit scope

- Workspace baseline before edits: `cargo clippy --all-targets --all-features --workspace -- -D warnings` (saved separately)
- Rollout-scope canonical baseline: `cargo clippy -p aspen-time --all-targets -- -D warnings`
