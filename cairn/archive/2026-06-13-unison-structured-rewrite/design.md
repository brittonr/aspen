## Context

With content-addressed artifacts, changes create new artifacts rather than mutating old ones. That makes runtime evolution safer, but users still need tools to find and update references across protocol manifests, schemas, policies, docs, effect manifests, and storage migration recipes. A structured rewrite system can produce upgrade plans instead of fragile text diffs.

Unison's structured find/replace is prior art. Molten's adaptation should operate on canonical Preserves values and artifact DTOs, integrate with policy/evidence gates, and feed upgrade sessions.

## Goals

- Search artifact graphs by canonical structure, not only text.
- Produce explicit rewrite plans with impacted artifacts and metadata pointers.
- Validate rewritten artifacts before any cutover.
- Integrate with executable transcripts, evaluation cache, and upgrade sessions.
- Preserve old artifacts and make rewrites auditable through receipts.
- Support both machine-readable patches and human-readable previews.

## Non-Goals

- Do not implement arbitrary code transformation for every source language in the first version.
- Do not mutate immutable artifact content in place.
- Do not allow text grep replacements to bypass canonical artifact validation.
- Do not automatically apply rewrites without policy admission and user/tool confirmation.
- Do not claim semantic equivalence unless supported by tests, policies, or verified predicates.

## Query model

A structured query should specify:

- artifact kinds and dependency scopes to search,
- Preserves/schema/manifest pattern,
- binding variables and constraints,
- identity mode requirements for schema refs,
- dependency traversal direction and depth,
- policy scope and visibility limits.

Query results should include artifact ids, path within canonical structure, bound variables, surrounding context, and reverse-dependency impact.

## Rewrite plan

A rewrite plan should include:

- source query artifact and hash,
- matched artifact ids and paths,
- replacement template or transformer artifact,
- expected new artifact ids if deterministic,
- impacted metadata pointers and upgrade-session tasks,
- validation requirements: schema checks, Trellis projectability, Wasm inspection, policy normalization, transcript reruns,
- rollback and cleanup notes,
- policy and evidence refs.

Applying a plan creates new artifacts and optionally proposes metadata moves. Existing artifacts remain addressable.

## Preview and validation

Preview should show:

- canonical structural diff,
- rendered text/doc diff when available,
- dependency impact set,
- changed schema/effect/capability surfaces,
- receipts that would be required,
- transcripts/tests selected for rerun.

Validation runs before cutover and may use cached deterministic results when valid.

## Integration

Upgrade sessions use rewrite plans as task sources. Typed storage uses rewrites to propose migration recipes. Choreography uses rewrites to update payload schemas or labels while checking projectability. Artifact docs use rewrites to update hash references and examples.

## Policy and evidence

Structured rewrite is a trust-boundary action when it creates installable artifacts or moves metadata. Policy should control which artifact kinds and namespaces a capability may rewrite. Receipts should cover query, preview, plan admission, artifact creation, validation results, and metadata application.

## Open Questions

- What is the first pattern language: Preserves pattern subset, Nickel-like selectors, or a custom registry query DSL?
- Should rewrite transformers run as Steel, Wasm, or pure Rust functions first?
- How should human approval be represented in receipts?
- Which semantic equivalence claims can Trellis or other verified predicates support?
