## ADDED Requirements

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
