## Why

Aspen already carries strong style and safety expectations, but default Clippy warnings still behave like advice instead of repository policy. A new contributor can clone the repo, run `cargo clippy`, and receive soft suggestions instead of hard feedback that matches the team's standards. That leaves lint expectations partially implicit and easy to ignore.

## What Changes

- Turn repository Clippy settings into enforced policy instead of warning-only guidance.
- Define the crate-level lint baseline Aspen requires for production crates, including `missing_docs`, `clippy::all`, `clippy::pedantic`, and `clippy::undocumented_unsafe_blocks`, while keeping the existing `clippy::module_name_repetitions` exception explicit.
- Add a repo-traveling `clippy.toml` that fixes key thresholds and exported-API policy so the same lint contract applies for every clone and CI run.
- Extend the policy to adjacent executable standards in the same flavor: rustdoc warnings become errors for the rollout scope, local and CI lint commands must stay semantically identical, all rollout-scope warnings become failures, additional lint exceptions must be narrowly justified instead of silently broad, and the repository must expose one canonical checked-in lint entrypoint for contributors and CI.
- Add Tiger Style-aligned guardrails around the lint policy: item-scoped suppressions by default, explicit unsafe-site inventory, threshold violations treated as real policy failures, no ambient per-user setup assumptions in the canonical entrypoint, consistent crate-root deny blocks across the rollout scope, negative-proof verification that representative violations fail, and bounded backlog tracking for deferred crates or approved suppressions.
- Document and verify the rollout path for bringing existing crates into compliance without relying on wiki-only guidance, including an explicit in-scope / deferred-crate inventory.

## Non-Goals

- Replacing Aspen's Tiger Style linting stack.
- Defining every possible Rust lint policy in this change.
- Introducing per-engineer local tooling conventions outside the checked-in lint configuration.

## Capabilities

### Modified Capabilities
- `tigerstyle-rollout`: make repository lint expectations executable through checked-in Clippy deny policy, explicit rollout inventories, negative-proof enforcement evidence, and local/CI parity requirements.
- `core`: require Aspen crates to surface missing docs, pedantic Clippy violations, threshold overruns, and rustdoc warnings as build errors rather than warnings.

## Impact

- **Files**: crate roots that participate in the enforced baseline, new top-level `clippy.toml`, and OpenSpec artifacts under `openspec/changes/archive/2026-04-23-enforce-clippy-policy/`.
- **APIs**: no runtime API changes; compile-time lint policy becomes stricter.
- **Dependencies**: no new runtime dependencies.
- **Testing**: verification will rely on saved `cargo clippy` and rustdoc-warning transcripts showing enforced failures are gone for the chosen rollout scope, negative-proof evidence that representative violations fail under the canonical entrypoint, unsafe-site inventory evidence where applicable, and proof that the same command semantics apply locally and in CI.
