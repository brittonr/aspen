# Molten World Commit Specification Delta

## Purpose

Provide one preview-first operator workflow that composes world-commit capabilities without hiding their owners, decisions, or claim boundaries.

## ADDED Requirements

### Requirement: World operator requests use explicit typed facts

r[molten.world_operator.plan] Every world workflow request MUST name exact commit, branch, expected generation, profile, policy, limit, authority-observation, and operation identities required by the selected actions. Planning MUST remain pure and compute a domain-separated BLAKE3 plan identity.

#### Scenario: Complete request is planned

- GIVEN all required immutable and mutable observations are supplied within bounds
- WHEN workflow planning runs
- THEN it MUST return one deterministic ordered operation graph and plan identity

#### Scenario: Request asks for latest head implicitly

- GIVEN a mutating request omits the expected head or generation
- WHEN workflow planning runs
- THEN Molten MUST reject the request before shell effects

### Requirement: Mutating commands are preview-first

r[molten.world_operator.preview_apply] Molten MUST preview every checkpoint, branch, run, promotion, import-publication, and retention mutation. Apply MUST require the exact plan identity and MUST recheck mutable facts inside each owning mutation boundary.

#### Scenario: Preview remains current

- GIVEN an operator submits the exact plan and all mutable observations still match
- WHEN apply runs
- THEN each component MAY execute its admitted operation in dependency order

#### Scenario: Head changes after preview

- GIVEN the branch generation changed after planning
- WHEN apply admission runs
- THEN the workflow MUST stop before using the stale head plan

### Requirement: The CLI is a thin composition root

r[molten.world_operator.composition] The `molten world` command family MUST call existing world cores and application services. It MUST NOT duplicate capture, merge, authority, effect, replay, replication, retention, or snapshot domain logic in CLI dispatch.

#### Scenario: Command plans a promotion

- GIVEN a valid promotion request
- WHEN CLI dispatch runs
- THEN it MUST delegate policy and transition decisions to their owning cores
- AND it MUST retain effect execution in explicit shell adapters

### Requirement: Operator commands have a closed typed surface

r[molten.world_operator.commands] Molten MUST provide typed checkpoint, branch, run, diff, conflicts, replay, simulate, verify, promote, export, import, and garbage-collection planning requests. Raw command strings and ambient credential lookup MUST NOT enter the workflow core.

#### Scenario: Raw shell command is supplied

- GIVEN a workflow request contains executable command text instead of a declared operation descriptor
- WHEN request validation runs
- THEN Molten MUST reject it as an unsupported authority surface

### Requirement: Aggregate receipts preserve component claims

r[molten.world_operator.receipt] A world workflow receipt MUST link ordered component plans, receipts, completion states, and the first blocker. It MUST preserve each component's evidence role and MUST NOT convert aggregate completion into whole-stack correctness, authority, or release eligibility.

#### Scenario: Workflow stops after a replay divergence

- GIVEN earlier operations completed and replay finds a divergence
- WHEN the workflow stops
- THEN the aggregate receipt MUST link completed receipts and the divergence receipt
- AND it MUST NOT report later promotion or export actions as complete

### Requirement: Dogfood covers one complete logical slice

r[molten.world_operator.dogfood] Molten MUST dogfood checkpoint, attenuated branch creation, deterministic run, successor capture, diff, conflict inspection, replay, verification, promotion reservation, export, import, and retention planning in one bounded logical workflow. It MUST separately test one exact opaque restore and replay profile.

#### Scenario: Logical fixture completes

- GIVEN the admitted logical fixture and deterministic adapter cohort
- WHEN the dogfood workflow runs
- THEN every operation MUST produce its expected plan and receipt linkage

#### Scenario: Opaque fixture requests semantic merge

- GIVEN the exact opaque fixture has divergent machine snapshots
- WHEN the workflow requests semantic merge
- THEN Molten MUST reject the operation without falling back to logical merge

### Requirement: Diagnostics remain bounded and redacted

r[molten.world_operator.diagnostics] Machine and human summaries MUST name stable operation, profile, blocker, and receipt references. They MUST NOT emit private keys, bearer tokens, raw environment values, unbounded state, or implicit host paths.

#### Scenario: Authority denial includes secret input

- GIVEN an adapter observes a secret while authority admission denies
- WHEN the operator summary is rendered
- THEN the summary MUST expose only the bounded denial class and safe references

### Requirement: Operator verification covers success and failure

r[molten.world_operator.verification] Molten MUST test complete workflows, stable plans, stale observations, denied authority, unresolved conflicts, uncertainty, incomplete capsules, unavailable profiles, raw commands, secret disclosure, and garbage-collection overclaims.

#### Scenario: Focused operator rail runs

- GIVEN positive and negative fixtures use the reviewed dependency cohort
- WHEN operator verification runs
- THEN it MUST report each supported or blocked profile and its bounded non-claims
