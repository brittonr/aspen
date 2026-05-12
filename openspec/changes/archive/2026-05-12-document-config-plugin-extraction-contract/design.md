## Context

`docs/crate-extraction.md` defers config/plugin APIs until the jobs/CI Nickel config seam identifies a stable reusable contract. The next useful step is documentation and policy structure, not readiness promotion.

## Goals / Non-Goals

**Goals:** eliminate owner/manifest ambiguity, document reusable contracts and examples, and add future evidence rails.

**Non-Goals:** publish crates, implement plugin host changes, or raise readiness in the same slice.

## Decisions

### 1. Manifest-first, evidence-next

**Choice:** Create a manifest and policy row while keeping readiness `workspace-internal`.
**Rationale:** The current gap is structural; readiness requires later compile/evidence work.

### 2. Separate API crates from host runtime

**Choice:** Treat `aspen-plugin-api` protocol/types separately from Hyperlight/host execution integration.
**Rationale:** Reusable plugin APIs should not inherit VM/runtime host dependencies by default.

## Risks / Trade-offs

**Premature API commitment** → Keep semver policy provisional and require standalone examples before promotion.
