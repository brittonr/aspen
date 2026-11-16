## Context

The issue offers two fixes: emit only computed checks and have consumers verify values, or document check lists as descriptive labels. Converting every family to computed statuses would change receipt bytes, canonical references and stored evidence across many families, which needs a versioned compatibility change per family.

## Decisions

### Decision: Document check lists as labels and pin the consumer boundary

**Choice:** take the issue's documentation option. The architecture and the check-set requirement state the label semantics, the helpers say they check names only, and tests pin that admission follows the decision even when every label reads `pass`.

**Rationale:** no current admission path relies on a constant `pass` status; decisions already carry the outcome. Documenting the boundary removes the misleading reading without rewriting evidence identities.

### Decision: Keep computed-status families as they are

**Choice:** families whose statuses are computed (node runtime source gates, Iroh endpoint admission) and consumers that require `pass` explicitly (authority admission, delivery idempotency) are left unchanged and named as exceptions.

**Rationale:** they already meet the issue's first option.

## Risks / Trade-offs

- Readers of older evidence may still over-read constant labels; the documentation is the mitigation.
- A future family that wants checks to carry evidential weight must compute them and have consumers verify values, as a versioned change.
