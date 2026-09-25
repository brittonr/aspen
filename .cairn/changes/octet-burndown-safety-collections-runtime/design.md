# Design: Octet burn-down, collection growth in runtime and adapters

## Context

The lint and the structural repair are the same as in `octet-burndown-safety-collections-validators`. Several runtime
sites read external or stored data, such as manifests, run indexes, directory listings, stored commits, and redb
records. For those, the growth is only as bounded as the data.

## Decisions

### Decision: Reuse admitted limits for new denials

**Choice:** Each new denial uses `crate::bounded::push_bounded` or an explicit length guard against a limit the caller
already admits: the cluster lifecycle item cap, the core run-artifact cap, the manifest `max-choices`, the policy
`max_members`, and the closure `max_closure_objects`. A new named constant is used only where no limit exists
(`MAX_WORLD_HEAD_CONFLICT_RECORDS`).

**Rationale:** A repeated or new magic number would let the output bound drift from the admitted input bound.

### Decision: Bounds that duplicate an earlier guard stay, and are tested end to end

**Choice:** Some limits are also enforced earlier: the core scheduler on `max-choices`, the traversal entry count on
`max_members`, and the closure walk on `max_closure_objects`. The collection-level bound stays as the local invariant.
Its tests check the observable behavior at the limit and one past it.

**Rationale:** The lint requires a local bound. The earlier guard fires first, so a denial is never weaker than before.

## No-spec classification

Accepted requirement text does not change. Semantic review inputs: the diff and the Octet, clippy, test, and receipt
evidence.

## Failure behavior

A new bound denies with the module's existing error type, before the collection is returned. No partial output is
returned.

## Risks / Trade-offs

- A cluster manifest, run index, or conflict set past its bound used to be accepted and is now denied. The limits
  are at or above the caps the downstream consumers already enforce.
