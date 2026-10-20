# Facet owner scopes verification

## Baseline census

Before this change, `RuntimeState` stored messages, assertions, and observers
in ordered sets keyed by actor strings. `cleanup_actor_scope` removed one
actor's entries and returned their refs; there was no facet record, parent
relation, ordered stop, tombstone, or crash-versus-stop distinction. The
flat-scope behavior is covered by the existing dataspace cleanup and
notification tests.

## Implementation

- `RuntimeFacet` records the facet id, owning actor, optional parent facet,
  running or stopped state, and optional stop-handler ref. The runtime snapshot
  includes the facet map.
- Assertions and observers carry an optional facet owner. Flat steps use
  `None`, preserving existing actor-scope behavior.
- `apply_facet_create` admits a child only under a running parent owned by the
  same actor, with bounded id/depth/count checks.
- `apply_facet_assert`, `apply_facet_observe`, `apply_facet_retract`, and
  `apply_facet_create` deny work aimed at a stopped facet before commit.
- `apply_facet_stop` stages one turn: running descendants are stopped
  depth-first, each facet's assertions and observers are removed, the facet is
  marked stopped, and its stop-handler ref is reported last. The tombstone
  remains, so the facet cannot restart.
- `cleanup_actor_scope` removes the same facet-owned state and facet records on
  the crash path without running or reporting a stop handler.
- New runtime events and turn actions encode facet creation, stop, and
  stop-handler reporting through the existing canonical receipt path.

## Validation

- `cargo test -p molten --lib dataspace`: 31 passed, 0 failed.
- `cargo test -p molten`: 1,572 lib tests plus all integration suites and
  doctests passed.
- `cargo check --workspace --all-targets`: pass.
- `cargo clippy --workspace --all-targets -- -D warnings`: pass.
- `cargo fmt --all --check`: pass.
- `cargo octet check --artifact-dir target/octet-facet-scope-3`: clean, 0
  findings, 0 warnings, 0 errors.
- Positive tests cover child-before-parent stop order, handler ordering,
  facet-owned assertion retraction, and flat actor cleanup.
- Negative tests cover late assertion, observer, child, and repeated stop
  denials, plus crash cleanup without handlers.

## Non-claims

Facets are an ownership and lifetime mechanism over the same dataspace state;
they grant no capability or policy authority. The core reports the stop-handler
ref and ordering, while actual handler execution remains shell-owned. A handler
that attempts a new assertion after the stop turn reaches the stopped-facet
denial; the core does not execute arbitrary handler code inside the pure state
machine. No whole-system, live-profile, or release claim is made.
