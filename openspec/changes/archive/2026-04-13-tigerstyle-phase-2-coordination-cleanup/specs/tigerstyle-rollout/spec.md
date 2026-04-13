## ADDED Requirements

### Requirement: Coordination crate promotion stays gated on bounded Tiger Style findings

Aspen SHALL keep `crates/aspen-coordination` out of the default Tiger Style
pilot until the coordination findings are inventoried by file and category.

Aspen SHALL reduce or explicitly isolate the coordination crate's ambient-clock,
compound-assertion, and panic-family findings before promoting the crate into
the default Tiger Style pilot.

Aspen SHALL save a fresh `scripts/tigerstyle-check.sh -p aspen-coordination --
--lib` transcript before adding `aspen-coordination` back to the default pilot
scope.

#### Scenario: Reviewing the deferred coordination crate

- **GIVEN** `aspen-coordination` is excluded from the phase-1 default pilot
- **WHEN** a maintainer prepares the phase-2 rollout change
- **THEN** the change SHALL inventory the current coordination findings by file
  and category
- **AND** it SHALL classify which findings need refactors versus explicit
  boundary ownership

#### Scenario: Promoting coordination into the default pilot

- **GIVEN** the coordination cleanup work has landed
- **WHEN** a maintainer proposes adding `aspen-coordination` back to the default
  Tiger Style pilot
- **THEN** the repo SHALL contain a saved rerun transcript for
  `scripts/tigerstyle-check.sh -p aspen-coordination -- --lib`
- **AND** the remaining warning set SHALL be bounded enough to review without
  blanket suppression
