# Compute Capacity Specification Delta

## ADDED Requirements

### Requirement: Offers separate qualified compute from workers
r[molten.compute_capacity.offers] Aspen MUST expose bounded versioned offers scoped to provider/source profile refs, host and device incarnation, topology generation, revision, logical observation and expiry, health and qualification refs. Compatibility dimensions with unknown evidence MUST remain unknown; a missing required dimension MUST NOT become a positive match. This service MUST NOT redefine fabric membership CPU/memory/storage semantics or claim model readiness or inference capacity.

#### Scenario: Two differently qualified hosts
- GIVEN two independently sourced host offers with distinct dedicated and unified accelerator memory and different runtime profiles
- WHEN a consumer evaluates a requested runtime against an unexpired offer
- THEN eligibility reports an explicit eligible, stale, unavailable, incompatible or insufficient-evidence decision.

### Requirement: Physical backing pools determine capacity
r[molten.compute_capacity.backing_pools] Aspen MUST map aliases, partitions and grouping views to explicit physical backing pools with one selected authority per pool. Physical totals, policy-allocatable quantities, observed use/free, committed charges and whether observations include those charges MUST remain distinct. An observation that includes charges MUST bind the charge cursor captured at observation, not delivery; active charges newer than that basis MUST remain debited. Missing observations MUST NOT imply zero use or full availability. Unsupported groupings MUST NOT be constructed from chip labels.

#### Scenario: Overlapping views share one chip
- GIVEN MMIO, remote chip, tray and overlapping mesh views whose claims resolve to the same chip pool
- WHEN a chip is already reserved through any view
- THEN an overlapping claim cannot spend that chip again, and an unenforced mesh is unsupported.

#### Scenario: Delayed debited observation
- GIVEN a pool observation was captured before a later reservation charged that pool
- WHEN the observation arrives after the reservation and reports free already net of older charges
- THEN the new charge remains debited and cannot be hidden by delivery-time accounting.

### Requirement: Reservation authority and idempotency
r[molten.compute_capacity.reservation_authority] Aspen MUST reevaluate current revision, incarnation, freshness, compatibility, label authority, grant validity and revocation, consumer/operation/attempt and owner generation before returning any protected activation intent. Admitted grant/label owners and an admitted logical clock MUST supply those facts independently of caller request fields. One serialized selected authority MUST atomically charge every requested physical pool or none and MUST NOT spend foreign-authority claims; a duplicate operation with matching owner MUST return its existing logical allocation. Serving deployments MUST charge resident resources once, even when referenced by multiple execution consumers on other hosts. An intent MUST be persisted before backend activation.

#### Scenario: Two consumers contend for one pool
- GIVEN independently fitting requests against one current pool
- WHEN its selected owner serially applies reservation transitions
- THEN only requests fitting the resulting state succeed, and a duplicate operation adds no charge.

#### Scenario: Request fields cannot forge admitted facts
- GIVEN a caller backdates the request and authors a valid grant or weaker label minimum while the admitted owner reports expiration, revocation or stronger policy
- WHEN the selected service evaluates reservation
- THEN it denies before any protected backend effect, and a foreign selected pool owner also denies the charge.

### Requirement: Uncertainty and release preserve charges
r[molten.compute_capacity.uncertainty_release] Aspen MUST keep claims charged or quarantined on drain, cancel, timeout, lost stop acknowledgement or backend disappearance. Release MUST require the matching consumer and owner generation plus affirmative backend-release evidence from the selected backend observation port; caller-authored evidence MUST NOT discharge a claim. A definitive no-effect activation refusal MAY retire the intent only after a durable no-effect transition. A stale owner MUST NOT release a successor. Logical fencing MUST be reported separately from actual backend enforcement. Retained released IDs MUST reject replay within a bounded FIFO horizon; an evicted ID becomes unknown and MAY be admitted by a new valid request.

#### Scenario: Stop acknowledgement lost
- GIVEN a charged allocation whose stop result is unknown
- WHEN cancellation or reconciliation is evaluated
- THEN the resource remains charged until matching affirmative backend release is established.

#### Scenario: Forged or absent backend observation
- GIVEN a quarantined allocation with no matching positive release evidence at the selected backend
- WHEN a caller requests reconciliation or supplies a mismatched assertion
- THEN the shell does not accept caller-authored evidence, does not repeat a possible stop and retains the charge.

#### Scenario: Bounded release replay
- GIVEN a released operation is retained in the durable FIFO tombstone horizon
- WHEN its operation ID is submitted again
- THEN replay is rejected until the ID is evicted, after which it is unknown and requires a fresh valid admission.

### Requirement: Readback has bounded qualification scope
r[molten.compute_capacity.readback] Aspen MUST expose bounded readback of offer freshness, health, qualification scope, allocation owner/generation/granularity/status and enforcement strength without secrets or raw backend handles.

#### Scenario: Inspect a quarantined claim
- GIVEN a fresh offer and an allocation whose backend release is uncertain
- WHEN inspect is evaluated at an explicit logical time
- THEN it reports quarantine, logical fencing and the recorded enforcement level separately.

### Requirement: Durable intent and canonical reconciliation
r[molten.compute_capacity.durable_reconciliation] Aspen MUST store bounded canonical packed Preserves state beneath an admitted durable root with a pinned selected owner, atomically compare current state when changing offers and reservations, and persist intent before a protected backend effect. On reopen, an unresolved intent MUST remain charged and MUST NOT trigger effect replay or a successful response without matching backend-owned evidence. Exact committed activation evidence MUST reconcile idempotently. Strict versioned public offer, allocation and readback records MUST reject wrong labels, arity and malformed refs; their identity MUST use BLAKE3 over canonical packed Preserves bytes.

#### Scenario: Interrupted backend activation
- GIVEN a reservation intent was durably written and a backend start may have happened without acknowledgement
- WHEN the store is reopened and the same operation is requested
- THEN it reports uncertainty without repeating the backend effect or spending the pool again.

#### Scenario: Ack lost after commit
- GIVEN activation or backend release evidence was committed durably before a response was lost
- WHEN the operation is inspected after reopen
- THEN the committed owner state is read back without false success or premature release.

#### Scenario: Exact activation replay after lost response
- GIVEN matching backend activation evidence was durably committed before its response was lost
- WHEN the store reopens and the selected backend reports that same evidence
- THEN reconciliation returns the existing allocation without repeating activation or changing its charge.

### Requirement: Deterministic fixtures exercise physical overlap
r[molten.compute_capacity.fixtures] Aspen MUST provide bounded positive and negative core fixtures for two incompatible hosts, dedicated and unified memory, overlapping aliases/partitions and independently authored synthetic Tenstorrent chip/tray/mesh views. Synthetic fixtures MUST NOT claim live device enforcement, actual TT grouping qualification, inference readiness or per-request throughput.

#### Scenario: Synthetic TT grouping is not inferred
- GIVEN an MMIO chip with remote chips, an overlapping tray and two shared-chip mesh views
- WHEN grouping is unsupported or its chip pool is already claimed
- THEN reservation returns unsupported or insufficient capacity without synthesizing another independent accelerator.
