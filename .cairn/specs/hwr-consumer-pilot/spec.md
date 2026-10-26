# Hwr Consumer Pilot Specification

## Purpose

Defines the `hwr-consumer-pilot` capability.

## Requirements

### Requirement: Molten owns the actor contract

r[molten.hwr_consumer_pilot.actor] The standalone pilot MUST define a Molten-owned actor world and export, execute an actual Component Model guest, and validate the exact actor-turn output in the consumer. Product actor semantics MUST NOT enter the reusable runtime producer.

#### Scenario: Admitted actor turn completes

- GIVEN the declared `molten:component-runtime@1.0.0/actor` world and `actor-turn` export
- WHEN input bytes `0700000029000000` are executed through the corrected component API
- THEN the outcome MUST be `Completed` with exactly `070000002a000000`
- AND actor id 7 MUST be preserved while counter 41 becomes 42.

#### Scenario: Malformed actor turn reaches the guest

- GIVEN an admitted component with the four-byte input `07000000`
- WHEN `actor-turn` executes
- THEN the normalized outcome MUST be `GuestDenied` with no successful output.

### Requirement: Runtime inputs and mechanisms are explicit and bounded

r[molten.hwr_consumer_pilot.boundary] The pilot MUST use admitted explicit engine feature policy and resource limits, BLAKE3 identity of actual component binary bytes, consumer-owned world/schema facts, exact qualified function names, empty admitted imports, and a positive finite deadline with an actively operated engine-bound epoch service. It MUST NOT use ambient WASI, optional-deadline bypass, core-module fallback, or production-host wiring.

#### Scenario: Exact component facts admit

- GIVEN component bytes encoded from the checked-in actor guest, the declared actor WIT identity, exact export name, and no imports
- WHEN the consumer prepares and runs the component under its admitted policy and limits
- THEN it MUST use `ComponentInvocation::prepare` and `run`
- AND consumed fuel MUST be positive and below the 10,000-fuel budget for the successful turn.

#### Scenario: Epoch service is absent

- GIVEN otherwise matching component facts and a finite budget but no epoch handle
- WHEN preparation runs
- THEN preparation MUST fail with `DeadlineDenied` before execution.

#### Scenario: Input exceeds the admitted carrier

- GIVEN the pilot's eight-byte maximum input and a nine-byte request
- WHEN invocation runs
- THEN the outcome MUST be `InputLimitExceeded` with no successful output.

### Requirement: Delivered dependency identity is immutable and external

r[molten.hwr_consumer_pilot.pin] Both producer crates MUST use one immutable external Git revision, matched by the pilot provenance record. The historical published pin MAY remain only as explicitly documented temporary working input while corrected publication is pending; it MUST NOT be reported as runnable corrected-API evidence. Delivered dependencies MUST NOT use placeholders, sibling paths, or development patches as release identities.

#### Scenario: Corrected candidate becomes available

- GIVEN a reviewed immutable producer revision exposing the corrected bounded component API
- WHEN the integration owner admits it for the pilot
- THEN both manifest producer revisions and the provenance revision MUST be changed together before recording smoke evidence.

#### Scenario: Only an old publication or local override exists

- GIVEN the published revision lacks the required API or the corrected API exists only via a development patch
- WHEN lifecycle evidence is assessed
- THEN immutable-publication execution evidence MUST remain incomplete.

### Requirement: Pilot evidence stays bounded and repeatable

r[molten.hwr_consumer_pilot.evidence] The governing runtime document MUST identify the maintenance owner, immediate outcome, durable contribution, exact standalone command, prerequisites, and non-claims. Lifecycle state MUST distinguish source implementation, observed smoke execution, and immutable publication. Runtime success MUST NOT grant production materialization trust, policy, authority, or release eligibility.

#### Scenario: Operator exercises the pilot

- GIVEN the corrected immutable candidate is pinned and required tools/source access are available
- WHEN `cargo run --manifest-path pilots/hwr-consumer/Cargo.toml` succeeds
- THEN evidence MAY record exact output, normalized denials, bounded fuel observation, command, and source identities
- AND it MUST remain pilot evidence rather than production adoption, Mantle trust, canonical Preserves, actor lifecycle, effect, replay, host-isolation, or whole-system proof.

#### Scenario: Source exists but execution is pending

- GIVEN the command assertions are implemented but have not executed against the corrected candidate
- WHEN tasks and documentation are updated
- THEN source work MAY be checked complete
- AND execution, publication, and unperformed lifecycle gates MUST remain unchecked and unclaimed.
