## Context

Commit `dbf254652` promoted Hermit/Uhyve only after a real Uhyve run accepted a valid `x86_64-unknown-hermit` marker image and the Aspen product-path receipt included `ASPEN_HERMIT_UHYVE_RUNTIME_HOST_EXECUTED`. The marker image was local proof material under `/tmp`, not a committed reusable fixture. The committed harness row therefore still requires `ASPEN_HERMIT_UHYVE_IMAGE=<valid Hermit image>`.

## Goals

- Make the Hermit proof fixture reproducible from the flake.
- Keep the distinction between fixture build and product-path execution proof explicit.
- Preserve secret-safe, bounded receipts and explicit opt-in gating.

## Non-Goals

- Default-running the real Uhyve proof on ordinary CI hosts.
- Replacing Uhyve, changing the `HermitUhyveWorker` product path, or accepting fake runners as proof.
- Vendoring a large opaque binary without source/provenance unless a later implementation task documents why source-built packaging is blocked.

## Decisions

### 1. Flake package owns the marker fixture

**Choice:** Expose a package such as `.#hermit-uhyve-marker` whose output contains a stable marker image path and a small metadata/provenance file.

**Rationale:** The proof row already has a reproducible Uhyve binary via `.#uhyve`; the remaining reproduction gap is the valid Hermit image. A flake package makes the proof command copy/pasteable without relying on `/tmp` state.

**Rejected:** Commit the `/tmp` binary directly. That would be opaque and would not prove toolchain/source reproducibility.

### 2. Package builds are prerequisite evidence only

**Choice:** Docs, harness metadata, and tests MUST keep fixture package success separate from runtime-host row proof.

**Rationale:** The runtime-host promotion rule requires Aspen-spawned execution through `JobManager`/`WorkerPool`; a build artifact only proves that the input image exists.

### 3. Build-std/toolchain friction is an implementation risk

**Choice:** Implementation may either package Uhyve's test-kernel source path with a pinned Rust/Hermit toolchain and vendored git dependencies, or introduce a smaller source-built marker crate, but it must keep provenance and verification deterministic.

**Risk:** Hermit target builds can require `-Zbuild-std`, `llvm-tools`, rust-src, git dependencies such as `hermit-rs`, and registry crates used by Rust's std build.

**Mitigation:** Add the package in a focused slice with a contract check first; if a fully source-built package is blocked, capture blocker evidence rather than landing an opaque binary and claiming proof.
