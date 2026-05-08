## Context

Promoted runtime-host rows were renamed away from stale `*-gap.ncl` manifests and a guard prevents future promoted rows from living in gap-named files. The next consistency step is a single operator-facing acceptance bundle check over all related surfaces.

## Goals / Non-Goals

**Goals:** catch drift between readiness docs, suite manifests, generated inventory, proof markers, fixture packages, and anti-overclaiming language.

**Non-Goals:** rerun expensive/gated runtime proofs by default or promote additional runtime-host rows.

## Decisions

### 1. Static bundle first

**Choice:** The acceptance bundle is a deterministic static/documentation/harness check by default.

**Rationale:** It should be cheap enough to run in normal local verification and should not require KVM/Uhyve/Hyperlight.

**Alternative:** A full proof rerun bundle was rejected as too expensive for the default rail.

### 2. Explicit proof-boundary assertions

**Choice:** The check must assert both positive anchors and negative non-proof language.

**Rationale:** Runtime-host readiness is easy to overclaim from package/build-only evidence.

## Risks / Trade-offs

**False confidence** → Name the check as acceptance-bundle consistency, not live proof execution.

**Brittleness** → Assert stable anchors and marker constants rather than full prose copies.
