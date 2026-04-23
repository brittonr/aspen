## Context

Aspen already treats many quality constraints as executable policy: Tiger Style lints, OpenSpec preflight, and feature-boundary checks all travel with the repository. Clippy is not yet held to the same standard. Rust's default lint behavior warns, which means the repository can express a preference without actually enforcing it.

The requested policy is intentionally simple and clone-local:

```rust
#![deny(missing_docs)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::undocumented_unsafe_blocks)]
#![allow(clippy::module_name_repetitions)]
```

paired with a checked-in `clippy.toml`:

```toml
avoid-breaking-exported-api = false
cognitive-complexity-threshold = 15
too-many-arguments-threshold = 5
type-complexity-threshold = 200
```

Aspen is large, so the main design problem is rollout scope: a repo-wide deny flip may surface substantial existing debt. The change therefore needs explicit adoption rules, target scope, and saved proof that the enforced slice is clean.

## Goals / Non-Goals

**Goals**
- Make Clippy policy executable from the repository checkout.
- Standardize the deny baseline each participating crate must declare.
- Check in a single `clippy.toml` so threshold policy is versioned with the repo.
- Make rustdoc warnings part of the same executable policy for the rollout scope.
- Treat rollout-scope warnings and threshold overruns as policy failures, not advisory feedback.
- Keep local lint behavior and CI lint behavior semantically identical.
- Provide one canonical checked-in lint entrypoint for contributors and CI.
- Keep suppressions narrow, explicit, and reviewable.
- Define verification expectations that prove `cargo clippy` teaches Aspen standards through compiler errors, not advisory warnings.

**Non-Goals**
- Replacing Tiger Style with Clippy.
- Solving every existing lint in the whole workspace in one unbounded sweep.
- Introducing hidden CI-only behavior that differs from local `cargo clippy` runs.

## Decisions

### 1. Use crate-root deny attributes as the enforcement mechanism

**Choice:** participating crates add the deny/allow block directly at the top of each crate root (`lib.rs` and/or `main.rs` as applicable).

**Rationale:** crate-root attributes are explicit, reviewable, and survive every local clone. They match the requested model of executable policy and let the compiler teach standards without any external wiki.

**Alternative considered:** rely only on CLI flags or workspace scripts. Rejected because that hides policy behind tooling entrypoints and does not travel as clearly with each crate.

### 2. Version threshold policy in top-level `clippy.toml`

**Choice:** add a repository-root `clippy.toml` containing the requested exported-API and complexity thresholds.

**Rationale:** this keeps policy clone-local and deterministic for every engineer and CI environment.

**Alternative considered:** duplicate thresholds in documentation or wrapper scripts. Rejected because those do not make the standards executable by default.

### 3. Roll out enforcement through a bounded documented scope

**Choice:** the change must inventory which crates adopt the deny block in the first pass and save evidence for that exact scope.

**Rationale:** Aspen's workspace is large enough that turning on pedantic deny everywhere without an inventory risks an unreviewable cleanup blast radius. The spec should require an explicit rollout inventory and a saved clean `cargo clippy` transcript for the targeted scope.

**Alternative considered:** require full-workspace adoption immediately. Rejected at spec time because the user asked for the OpenSpec, not an assumption that all existing crates are already clean.

### 4. Treat documentation coverage as part of standards enforcement

**Choice:** `missing_docs` is part of the required deny block for participating crates, and verification must prove those crates pass under that stricter setting.

**Rationale:** the request explicitly frames these attributes as standards enforcement rather than style advice. Documentation completeness therefore becomes part of the executable contract.

### 5. Keep local and CI lint semantics identical

**Choice:** the change must define one canonical enforcement command (or small fixed command set) for the rollout scope, and CI must run that same semantic check rather than a stricter hidden variant.

**Rationale:** executable policy only teaches contributors reliably if the command they run locally matches what the repository enforces in automation.

**Alternative considered:** allow CI-only strictness. Rejected because that recreates hidden policy.

### 6. Treat warnings and configured thresholds as hard policy

**Choice:** the rollout-scope canonical entrypoint must fail on warnings, including configured Clippy threshold overruns, rather than surfacing them as advisory output.

**Rationale:** this is the core “enforcing, not advising” behavior the change is trying to achieve.

### 7. Narrow lint exceptions instead of broad suppression

**Choice:** beyond the standard `clippy::module_name_repetitions` exemption, additional `#[allow(...)]` uses in the rollout scope must be item-scoped by default, module-scoped only when item scope is impractical, and broad crate-wide suppressions are not the default escape hatch.

**Rationale:** deny policy loses value if it immediately turns into unreviewed blanket allows.

### 8. Inventory unsafe sites explicitly

**Choice:** if a rollout-scope crate contains `unsafe`, verification must save an inventory of those sites and show that the corresponding safety comments satisfy the enforced policy.

**Rationale:** Tiger Style prefers explicit exceptional boundaries; `unsafe` deserves the same reviewable treatment.

### 9. Expose one canonical checked-in lint entrypoint

**Choice:** the change must name one repository-owned command, script, or Nix target as the canonical rollout-scope lint entrypoint, and both contributor guidance and CI enforcement must route through that entrypoint or the same semantics it defines, without depending on ambient per-user shell aliases, editor settings, or untracked local configuration.

**Rationale:** executable policy is strongest when a new engineer has one obvious thing to run and gets the same answer automation gets.

**Alternative considered:** document several equivalent commands without a canonical path. Rejected because that increases drift and ambiguity.

### 10. Keep rollout-scope crate policy text uniform

**Choice:** rollout-scope crates should use the same crate-root deny block text unless a documented exception justifies a deviation.

**Rationale:** uniform policy text makes drift obvious and review mechanical.

### 11. Save both positive and negative enforcement proof

**Choice:** verification artifacts must include before/after or baseline/final `cargo clippy` transcripts for the rollout scope, rustdoc warning enforcement evidence for the same scope, rollout inventory artifacts for in-scope and deferred crates, canonical-entrypoint evidence, representative negative-proof evidence showing the canonical path fails on a known violation, unsafe inventory evidence where applicable, plus source diffs showing the deny block and `clippy.toml` landed.

**Rationale:** Aspen OpenSpec completion rules require durable repo evidence, not chat-only claims.

## Risks / Trade-offs

- **Large existing lint debt** → pedantic deny may require non-trivial cleanup before adoption. Mitigation: require an explicit rollout inventory, named deferred crates, owners or follow-up references for backlog items, and bounded initial scope.
- **Crate-root churn** → adding the same deny block to many crates is repetitive. Mitigation: keep the exact block standardized and reviewable.
- **Documentation burden** → `missing_docs` and rustdoc warning denial may slow rollout for internal crates. Mitigation: adoption scope must be intentional and verified, not implied.
- **Allow-sprawl pressure** → contributors may try to bypass strictness with broad suppressions. Mitigation: make exception policy explicit, item-scoped by default, and reviewable.
- **Ambient-tooling drift** → local convenience aliases or ad-hoc commands can diverge from repo policy. Mitigation: require one checked-in canonical entrypoint with no ambient setup assumptions.
