# Proposal: Admit facet owner scopes

## Why

`docs/architecture.md` names facets in the public runtime model ("actors, entities, facets, assertions, retractions,
`Observe` patterns, and turns"), and the accepted requirement `molten.runtime_spine.assertion_lifetimes` already lists
facets as assertion owners. No facet type exists in the source. The only occurrences are a traceability comment and a
test in the reference harness (`src/runtime/dataspace/syndicate.rs:156`).

Owner scopes are flat actor strings with one `cleanup_actor_scope` step, so three rules that the manual states
directly have no representation: a facet "share[s] its fate" with its assertions, "a stopped facet never starts running
again", and crash skips stop handlers (`05-glossary.md → Facet`). Today a child scope cannot be stopped without
stopping its actor, stop order is unspecified, and an orderly stop is indistinguishable from a crash.

## What Changes

- Add a facet owner scope as a nested lifetime owner over assertions, observers, and child facets.
  r[molten.runtime_spine.facet_owner_scopes]
- Order every ordered stop: stop child facets first, then retract this facet's assertions and observers, then run the
  facet stop handler, then continue to the parent.
- Make stop permanent: a stopped facet accepts no new assertion, observer, or child, and it never restarts. Late work
  aimed at a stopped facet fails closed before commit.
- Keep the crash path distinct: a crash runs no stop handler, and owner-scope cleanup still retracts the facet's
  assertions and observers.
- Keep actor-scope cleanup working for actors that never create a facet.

## Impact

- **Files**: `src/runtime/dataspace/state.rs`, `src/runtime/dataspace/mod.rs`, `src/runtime/turn/mod.rs`,
  `src/runtime/dataspace/tests.rs`, `docs/architecture.md`.
- **Testing**: an ordered-stop trace for a two-level tree; positive rerun of actor-scope cleanup for facet-free
  actors; negative tests for restart after stop, assertion after stop, and handler assertions during a crash path;
  `cargo test -p molten` before and after; focused Clippy.
- **Non-goals**: no entity-level addressing, no scheduler change, no authority change, no new receipt schema, and no
  claim that facets grant capability.
