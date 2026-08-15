## Context

The existing flake intentionally maps private OnixResearch Cargo git dependencies to local source paths so Nix builders do not need SSH access. That boundary is useful, but the current expression records one user's checkout paths directly. The pre-commit Cairn validation hook has the same portability problem, and `rust-toolchain.toml` uses a floating channel even though release evidence should be tied to a stable toolchain identity.

## Decisions

### Config path resolution is explicit and relocatable

**Choice:** Define a small set of reviewed path inputs for sibling repositories and local source checkouts. Defaults may target the common `../cairn` and sibling-workspace layout, but user-specific absolute paths are only allowed in local untracked overrides or explicit environment variables.

**Rationale:** The repo remains easy to use in the OnixResearch workspace without baking one operator's home directory into reviewed config.

### Release toolchains are pinned

**Choice:** Use a dated Rust nightly or otherwise exact toolchain identity for release and Nix evidence. Exploratory local shells may remain more flexible only when they are clearly excluded from release evidence.

**Rationale:** Formatter, Clippy, Rust metadata, and unit2nix output can drift under a floating nightly.

### Cargo/Nix source pins get a pure drift check

**Choice:** Add a pure parser/decision core that compares the Cargo.lock git revisions for private dependencies with the Nix local-source map. The shell owns reading files and rendering diagnostics.

**Rationale:** The local source map intentionally duplicates source identity. A deterministic check makes the duplication safe to review.

### Nix check values are named

**Choice:** Extract repeated VM addresses, live event limits, retry/attempt bounds, profile names, and timeout values into named constants or small imported modules as files are touched.

**Rationale:** The flake is evidence-heavy and long; named constants make config changes reviewable without changing the check semantics.

## Validation strategy

- Run a new focused config lint/drift check with positive and negative fixtures.
- Run `cargo fmt --check` after touching Rust helper code.
- Run the existing Nickel export drift gate and nextest-config gate.
- Run Cairn proposal/design/tasks gates for this change.

## Non-claims

Passing portability config checks does not prove runtime correctness, release eligibility, source-gate trust, deployment trust, or that a local checkout's sibling repos are semantically current. It only proves the reviewed config no longer depends on hidden user-local path assumptions and that duplicated pins agree.
