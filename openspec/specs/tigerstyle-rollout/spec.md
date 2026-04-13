# tigerstyle-rollout Specification

## Purpose

Define how Aspen promotes high-noise Tiger Style rollout families one family at
a time, with per-family inventories, bounded rerun transcripts, and explicit
exceptions instead of blanket allows.

## Requirements

### Requirement: High-noise Tiger Style families are adopted one family at a time

Aspen SHALL promote high-noise Tiger Style safety families one family at a
time instead of enabling the full backlog at once.

For each promoted family, Aspen SHALL build a per-crate inventory, remove or
explicitly justify the highest-risk findings, and save a rerun transcript before
adding that family to the default Tiger Style path.

Aspen SHALL not use blanket allows to claim a family is ready for rollout.

#### Scenario: Starting the next safety-family cleanup pass

- **GIVEN** `ignored_result`, `no_unwrap`, `no_panic`, `unchecked_narrowing`,
  and `unbounded_loop` are still deferred from Aspen's default Tiger Style path
- **WHEN** a maintainer starts phase 3 cleanup
- **THEN** the change SHALL name the first family being promoted
- **AND** it SHALL include a per-crate inventory for that family
- **AND** it SHALL identify the highest-risk findings targeted in that slice

#### Scenario: Promoting a cleaned-up safety family

- **GIVEN** one deferred safety family has been triaged and reduced
- **WHEN** a maintainer proposes enabling that family in Aspen's default Tiger
  Style path
- **THEN** the repo SHALL contain a saved rerun transcript for that family
- **AND** the transcript SHALL show a bounded remaining warning set
- **AND** any remaining exceptions SHALL be explicit and justified rather than
  blanket suppression

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
