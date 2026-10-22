# Runtime spine: service state union delta

## ADDED Requirements

### Requirement: Service state is a union of derived assertions
r[molten.runtime_spine.service_state_union] Molten MUST project committed service state into a state set of `started`, `ready`, `complete`, `failed`, and user-defined values, MUST treat `ready` as implying `started`, MUST report a normal one-shot finish as `complete`, MUST derive an `up` alias over `ready` or `complete`, and MUST NOT let the projection drive lifecycle transitions or grant authority.

#### Scenario: One-shot service completes
- GIVEN a one-shot service that exits normally
- WHEN its committed state is projected
- THEN the state set contains `complete` and does not report `failed`.

#### Scenario: Ready implies started
- GIVEN a service that reported readiness
- WHEN its state set is read
- THEN both `started` and `ready` are present.

#### Scenario: Dependent waits for up
- GIVEN a dependent that declares a dependency on `up`
- WHEN the dependency service reaches `ready` or `complete`
- THEN the dependent proceeds, and it does not proceed while only `started` is present.

#### Scenario: User-defined state satisfies no built-in dependency
- GIVEN a service that asserts a user-defined state value
- WHEN a dependent declares a dependency on `ready`, `complete`, or `failed`
- THEN the dependency is not satisfied by the user-defined value.

#### Scenario: Projection never transitions the service
- GIVEN any state set produced by the projection
- WHEN the lifecycle FSM evaluates its next event
- THEN the next transition is decided by FSM and lifecycle evidence alone.
