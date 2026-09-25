# Node-core resource-bound source checkpoint, not startup approval

Producer cwd: `/home/brittonr/git/OnixResearch/molten-worktrees/node-content-service`. Rust source/documentation revision: `e7e722569b44b31416b3514157c0f3f19432c5d4` on `feat/molten-node-content-service`. No source selection, Cargo dependencies, Octet pin or severity, baseline, public signature, stored format, operator cohort, startup guard, or Nix input changed. Both `molten-core` and `molten-node-core` compile the same borrowed pure-source files.

The prior corrected-library diagnostic at `../standalone-node-corrected-diagnostic/` identified nine `unbounded_collection_growth` reports on its earlier source revision, not a current gate count. Classification and current behavior:

| Reported site | Assessment and source decision |
| --- | --- |
| `content_store_adapter/preflight.rs:49` | Genuine: oversized manifests previously reached transform diagnostics and required-ref cloning. Existing profile `max_chunk_count` now denies first; cancellation remains terminal and no refs are materialized. |
| `content_store_adapter/validation.rs:18,142` | Guarded false positives: each loop walks the seven-element `REQUIRED_CONTENT_NON_CLAIMS` fixed array; no source changes for these sites. |
| `content_store_adapter/validation.rs:49` | Genuine: direct public descriptor validation could append per-malformed-chunk diagnostics without a profile. Reject a chunk count impossible for strictly positive chunk lengths (`chunks.len() > total_length`), then stop at the first malformed chunk. Direct range selection rejects impossible counts/zero-length chunks before cloning refs. A structurally valid large direct range has no profile parameter; no arbitrary new size ceiling was imposed. |
| `fabric/port.rs:253` | Genuine: over-128 descriptor count previously still scanned and could append unbounded duplicate/malformed issues. Return the existing `TooManyPorts` issue before traversal. Also stop per-field validation at the existing 64-element list bound, including nested non-claims; no unbounded duplicate scan or per-value diagnostics after overlimit. |
| `fabric_crypto_identity/model.rs:381` | Guarded false positive: input count over 256 returns before the collection; subsequent required list is a fixed six-element array. No change. |
| `fabric_durability/mod.rs:547` | Guarded false positive: required non-claims are a fixed eight-element array; existing `validate_unique` limits supplied collections to 4096 before sorting. No change. |
| `fabric_durability/transition.rs:475,483` | Genuine: public snapshot/effect maps could append unbounded recovery issues, and an oversized unresolved ID could be cloned into a report. Cap cumulative issues from log gaps, snapshots and effects at the existing 4096-item domain bound, append one `CollectionLimitExceeded`, deny regardless of repair/quarantine permission, and never clone an overlong ID. |

Verification against the source before evidence-only files were added:

```text
PATH=/nix/store/c7qcfda063a1d95gfprn2xlhcvrpp1fl-rust-default-1.98.0-nightly-2026-05-26/bin:$PATH RUSTC_WRAPPER='' RUSTC_WORKSPACE_WRAPPER='' cargo test --locked --offline -p molten-node-core -p molten-node-runtime -p molten-node-host -p molten-core
  exit 0; 918 passed across 20 suites
PATH=/nix/store/c7qcfda063a1d95gfprn2xlhcvrpp1fl-rust-default-1.98.0-nightly-2026-05-26/bin:$PATH RUSTC_WRAPPER='' RUSTC_WORKSPACE_WRAPPER='' cargo test --locked --offline -p molten --lib
  exit 0; 1361 passed
PATH=/nix/store/c7qcfda063a1d95gfprn2xlhcvrpp1fl-rust-default-1.98.0-nightly-2026-05-26/bin:$PATH RUSTC_WRAPPER='' RUSTC_WORKSPACE_WRAPPER='' cargo test --locked --offline -p molten --test cliharness cli_cluster_
  exit 0; 4 passed
PATH=/nix/store/c7qcfda063a1d95gfprn2xlhcvrpp1fl-rust-default-1.98.0-nightly-2026-05-26/bin:$PATH RUSTC_WRAPPER='' RUSTC_WORKSPACE_WRAPPER='' cargo clippy --locked --offline -p molten-node-runtime -p molten-node-core -p molten-node-host -p molten-core -p molten --all-targets -- -D warnings
  exit 0
PATH=/nix/store/c7qcfda063a1d95gfprn2xlhcvrpp1fl-rust-default-1.98.0-nightly-2026-05-26/bin:$PATH RUSTC_WRAPPER='' RUSTC_WORKSPACE_WRAPPER='' cargo build --locked --offline -p molten-node-runtime --bin molten-node
  exit 0
```

Direct compiled CLI: `/home/brittonr/.cargo-target/debug/molten-node init --state-root /tmp/molten-node-resource-smoke.shNKNo/state --node-id node:resource-smoke --config-out /tmp/molten-node-resource-smoke.shNKNo/config.preserves` exited 0 and wrote the local identity/configuration; `molten-node run --state-root /tmp/molten-node-resource-smoke.shNKNo/state --startup-out /tmp/molten-node-resource-smoke.shNKNo/startup.json` exited 1 with `node-startup-source-gate-required: real Octet evidence admission is not wired; synthetic startup evidence is forbidden`. Directory inspection showed no `startup.json`. No production listener, VM, native replay, or promotion occurred.

`git diff --check` passed. Selective `rustfmt --edition 2024 --check --config skip_children=true` across the six edited Rust files returned 1: five differences were inherited pre-existing formatting in `fabric/port.rs` and `fabric_durability/{transition,tests}.rs`; two differences in newly written lines were corrected surgically. Neither selective nor full-workspace formatting pass is claimed. Source-only formatting did not change behavior after the passing suite. The pinned strict Octet hook was **not rerun for this source revision**: its prior 11 host findings and the alternate library's 287 node-core findings remain diagnostic history. The locked Radicle `executable-extent-src` input was previously unavailable; no Nix package realization or approved source/build/binary cohort is claimed. The producer Cairn and consumer VM tasks remain open.
