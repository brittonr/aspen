# Fabric Simulation Delta

## ADDED Requirements

### Requirement: Simulated transport state changes delivery
r[molten.fabric_simulation.stateful_transport] The simulated transport adapter MUST own pending transmissions with eligibility times. A delay fault MUST move an event's eligibility, a drop fault MUST prevent delivery, and a partition MUST block selected communication until it heals. Simulated completion events MUST enter scheduling through the same boundaries as live-shell events.

#### Scenario: Delay postpones delivery
- GIVEN a transport delay fault is active for one event
- WHEN the event's eligibility time is evaluated
- THEN the event is not eligible before the delayed tick and becomes eligible at it.

#### Scenario: Dropped event never delivers
- GIVEN a transport drop fault targets an event
- WHEN the schedule runs past the event's original eligibility
- THEN the destination never observes the event and the adapter records the drop.

#### Scenario: Partition blocks and heals
- GIVEN a partition blocks communication between two nodes
- WHEN a message targets the blocked destination and then the partition heals
- THEN messages before the heal are not delivered and messages after the heal are delivered.

### Requirement: Simulated storage state defines what survives
r[molten.fabric_simulation.stateful_storage] The simulated storage adapter MUST distinguish submitted operations, completed operations, and the recoverable durable image. A completion fault MUST hold an operation unapplied, and recovery after a crash MUST reconstruct service state from the durable image only.

#### Scenario: Crash loses unacknowledged-in-image writes
- GIVEN a storage completion is delayed so a write is submitted but not applied
- WHEN the service crashes and recovers
- THEN the recovered state equals the durable image and does not include the unapplied write.

#### Scenario: Delayed completion changes acknowledgment
- GIVEN the same workload with and without a delayed storage completion
- WHEN the service acknowledges before a crash
- THEN the acknowledged outcome can differ between the two runs, and a fault that only changes a receipt hash does not satisfy this requirement.

### Requirement: Schedules explore, replay, and minimize over real execution
r[molten.fabric_simulation.causal_exploration] The reference runner MUST select among causally eligible events using recorded seeded choices. Replay MUST compare virtual time, generations, semantic outputs, and choice records and stop at the first mismatch. Minimization MUST rerun each candidate and retain it only when the same failure fingerprint reproduces.

#### Scenario: Different seeds follow different schedules
- GIVEN two different seeds over the same world manifest
- WHEN exploration runs twice
- THEN at least one recorded choice differs and each run is internally deterministic.

#### Scenario: Replay detects a changed outcome
- GIVEN a recorded run and a replay whose semantic output differs at one position
- WHEN replay comparison runs
- THEN it reports the first mismatching field and does not report success.

#### Scenario: Minimization keeps only reproducing candidates
- GIVEN a candidate workload whose rerun does not reproduce the failure fingerprint
- WHEN minimization evaluates the candidate
- THEN the candidate is discarded and the retained minimized case reproduces the original failure.
