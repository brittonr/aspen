## Context

Aspen has many targeted verification commands, but contributors need one bounded confidence rail that is broader than a single focused test and cheaper than full dogfood or gated VM proof reruns.

## Goals / Non-Goals

**Goals:** define a deterministic quick rail with structured summary, selected check list, and explicit skipped-gated-proof notes.

**Non-Goals:** replace `nix flake check`, full dogfood, or ignored/gated KVM/Uhyve/Hyperlight proofs.

## Decisions

### 1. Compose existing checks

**Choice:** The rail should call existing canonical checks where possible rather than inventing new semantic assertions.

**Rationale:** It reduces maintenance and keeps results recognizable.

### 2. Explicit non-proof output

**Choice:** The rail must state which expensive/gated proofs were not run.

**Rationale:** Quick confidence should not be confused with runtime-host or production acceptance.

## Risks / Trade-offs

**Runtime cost** → Start with a bounded quick profile and allow future expansion through evidence.

**Overclaiming** → Include support-level and skipped-proof language in the summary/receipt.
