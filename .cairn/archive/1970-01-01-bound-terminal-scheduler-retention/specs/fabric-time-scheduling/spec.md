# F11 finite terminal retention

## ADDED Requirements

### Requirement: Terminal state has a finite named bound

r[molten.audit_f11.retention]
Molten MUST bound terminal records through a named limit derived from the admitted capacity contract.
Molten MUST reclaim terminal records deterministically without reclaiming active occurrences or widening initialized capacity.

#### Scenario: Repeated completion remains bounded
- GIVEN active limit one and an admitted finite terminal-retention bound
- WHEN successive fresh occurrences wake, run, and complete beyond that retention bound
- THEN retained terminal records stay within the bound and active work remains admissible within physical capacity

#### Scenario: Invalid retention capacity denies activation
- GIVEN the retention plan is unrepresentable or exceeds admitted physical capacity
- WHEN capacity admission runs
- THEN activation denies without a smaller hidden scheduler or partial state

### Requirement: Eviction cannot permit ABA resurrection

r[molten.audit_f11.freshness]
Molten MUST bind each occurrence to an explicit fresh issuance sequence and generation fence independent of retained tombstones.
Molten MUST reject old occurrence admission after eviction without an unbounded retired-identity set.

#### Scenario: A fresh occurrence reuses a logical name
- GIVEN an old terminal occurrence is evicted and a fresh issuance sequence exceeds the generation high-water fence
- WHEN admission names the same logical runnable with the fresh occurrence identity
- THEN only the fresh occurrence becomes eligible

#### Scenario: An old callback arrives after eviction
- GIVEN an old occurrence is evicted and its logical name now identifies different work
- WHEN its Wake, Complete, Yield, Block, or Cancel callback arrives
- THEN the old callback cannot create, resume, complete, or mutate the fresh occurrence

### Requirement: Retention arithmetic and rejection are atomic

r[molten.audit_f11.atomicity]
Molten MUST use checked counts, capacity calculations, and occurrence-sequence advancement.
Molten MUST preserve state on rejected issuance, stale input, arithmetic error, or invalid transition.

#### Scenario: Cleanup reclaims eligible terminal records
- GIVEN a retired generation contains terminal and active records
- WHEN authorized cleanup cancels its active work and applies retention
- THEN the result respects the named bounds and cannot affect another active generation

#### Scenario: Fresh occurrence issuance is exhausted
- GIVEN the occurrence sequence cannot advance
- WHEN a new occurrence requests admission
- THEN admission returns an explicit error without wrap, generation fallback, eviction side effects, or callback execution

### Requirement: Replay and evidence preserve occurrence meaning

r[molten.audit_f11.validation]
Molten MUST cover retention, eviction, callback fences, adapters, and replay with positive and negative normal repository tests.
Molten MUST distinguish executed F11 accumulation evidence from unexecuted fence claims and reject incompatible legacy identity interpretation explicitly.

#### Scenario: Matching replay preserves reclamation
- GIVEN identical admitted occurrence inputs, profile, and initial state
- WHEN replay runs across terminal eviction
- THEN it reproduces occurrence selection and deterministic reclamation without unbounded history

#### Scenario: A legacy callback lacks a fence
- GIVEN a callback cannot identify its occurrence under the admitted compatibility contract
- WHEN an adapter decodes it
- THEN admission denies without mapping it to current work or inventing historical receipts
