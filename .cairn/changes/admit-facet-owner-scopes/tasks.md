# Tasks: Admit facet owner scopes

## Model

- [ ] [serial] Record the current owner-scope census: which runtime records are owned by an actor string, and which callers need nested lifetimes first. r[molten.runtime_spine.facet_owner_scopes]
- [ ] [serial] Add the facet record, the parent relation, and facet ownership fields for assertions, observers, and child facets. r[molten.runtime_spine.facet_owner_scopes]

## Stop and crash behavior

- [ ] [serial] Implement the ordered stop action: children first, then this facet's assertions and observers, then the stop handler, with the tombstone recorded in the same turn. r[molten.runtime_spine.facet_owner_scopes]
- [ ] [serial] Implement the crash path: retract facet-owned state without running a stop handler. r[molten.runtime_spine.facet_owner_scopes]

## Validation

- [ ] [parallel] Add positive tests: a two-level facet tree stops in child-before-parent order with a recorded trace, and facet-free actor cleanup behaves exactly as before. r[molten.runtime_spine.facet_owner_scopes]
- [ ] [parallel] Add negative tests: a stopped facet never restarts, a late assertion or child aimed at a stopped facet denies before commit, and a stop handler cannot re-assert during the stop turn. r[molten.runtime_spine.facet_owner_scopes]
- [ ] [serial] Run `cargo test -p molten` and focused Clippy before and after the change, then the workspace checks the repository requires for runtime edits. r[molten.runtime_spine.facet_owner_scopes]
- [ ] [serial] Update `docs/architecture.md` with the facet ownership, stop order, permanence, and crash rules, and state the no-extra-authority non-claim. r[molten.runtime_spine.facet_owner_scopes]
