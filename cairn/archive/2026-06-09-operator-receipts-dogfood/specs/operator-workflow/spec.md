## ADDED Requirements

### Requirement: Operator dogfood receipts are canonical ledger artifacts
r[molten.dogfood.operator_receipt_schema] Molten MUST represent local dogfood operator receipts as canonical Preserves artifacts that bind run/workflow identity, config or policy refs, node identity refs, state hashes, child receipt refs, status, replay status, diagnostics, and redaction metadata where applicable.

#### Scenario: Final dogfood receipt binds child evidence
- GIVEN a local dogfood workflow completes
- WHEN the final dogfood report is written
- THEN the canonical report binds workflow, checkpoint, child receipt, gate, repro, status, diagnostics, and redaction-gate evidence refs

### Requirement: Dogfood receipt CLI readback
r[molten.dogfood.receipts_cli] Molten MUST expose operator receipt commands that list, show, validate, and export local dogfood receipt artifacts from the content-addressed evidence ledger.

#### Scenario: Operator validates a local dogfood receipt
- GIVEN a local dogfood run has imported operator artifacts into its ledger
- WHEN the operator runs receipt list, show, validate, and export commands for the dogfood report ref
- THEN Molten reads the canonical ledger artifact, validates the supported operator receipt schema, renders a non-normative summary, and exports canonical Preserves bytes

### Requirement: Receipt rendering is redaction-aware
r[molten.dogfood.redaction] Receipt list, show, validate, and export commands MUST avoid treating logs or unredacted text as authority and MUST render summaries as redaction-aware non-normative views over canonical Preserves receipts.

#### Scenario: Receipt export avoids log trust
- GIVEN an operator exports a local dogfood receipt
- WHEN Molten writes the exported artifact
- THEN the exported receipt remains canonical Preserves evidence
- AND rendered summaries and logs remain auxiliary views rather than primary evidence

### Requirement: Logs are auxiliary evidence only
r[molten.dogfood.no_logs_as_evidence] Molten MUST document that logs and CLI summaries are auxiliary operator aids; canonical receipts, traces, and content refs are the primary evidence for dogfood decisions.

#### Scenario: Operator inspects dogfood output
- GIVEN a dogfood run prints CLI status text
- WHEN release review evaluates the run
- THEN review uses canonical dogfood report, release gate, Nix evidence, verification receipt, trace, and content refs instead of log text

### Requirement: Local dogfood command remains the vertical slice
r[molten.dogfood.local_command] Molten MUST provide a local dogfood command that runs the deterministic local-node workflow and writes canonical report and release-gate artifacts.

#### Scenario: Local dogfood command completes
- GIVEN an empty explicit state root
- WHEN `molten dogfood local-node` runs
- THEN it writes a canonical dogfood report and release gate receipt for operator review

### Requirement: Local dogfood exercises runtime boundaries
r[molten.dogfood.vertical_slice] The local dogfood workflow MUST exercise config or policy refs, node identity, artifact installation, handler or service binding, local dataspace exchange, receipt storage, transcript or repro execution, and cleanup or retention review evidence.

#### Scenario: Dogfood report covers the vertical slice
- GIVEN the local dogfood workflow succeeds
- WHEN the report is parsed
- THEN it includes mandatory step evidence for startup, service or handler execution, remote-shaped delivery, job execution, catalog/readback, repro verification, retention review, and shutdown

### Requirement: Local dogfood state can be preserved for inspection
r[molten.dogfood.leave_running] Molten SHOULD allow local dogfood state and ledger artifacts to remain available for operator inspection after the workflow completes.

#### Scenario: Operator inspects preserved state
- GIVEN a local dogfood workflow ran with an explicit state root
- WHEN the command exits
- THEN the state root ledger remains available for receipt list, show, validate, and export commands

### Requirement: Dogfood final receipt summarizes outcome
r[molten.dogfood.final_receipt] Molten MUST store a final dogfood report receipt that records success or failure with child receipt refs, workflow refs, checkpoint refs, final state refs, final status, and diagnostics.

#### Scenario: Final report denies incomplete evidence
- GIVEN a mandatory dogfood step lacks canonical receipt evidence
- WHEN the final dogfood report is built
- THEN the report decision is `deny` and diagnostics name the missing evidence

### Requirement: Dogfood replay status is validated
r[molten.dogfood.replay_validation] Dogfood reports MUST require deterministic or recorded replay status for mandatory release evidence and MUST include first-divergence diagnostics when replay-bound verification fails.

#### Scenario: Non-replayable mandatory step denies release evidence
- GIVEN a mandatory dogfood step is marked non-replayable
- WHEN the report is evaluated
- THEN the dogfood report denies release evidence before a release gate is accepted

### Requirement: Cluster-backed receipt readback is planned but not required locally
r[molten.dogfood.cluster_readback_plan] Molten MAY add cluster-backed receipt readback later, but the local dogfood receipt CLI MUST work without production cluster storage.

#### Scenario: Local readback works before cluster storage
- GIVEN only a local dogfood evidence ledger exists
- WHEN receipt readback commands run
- THEN they operate on the local content-addressed ledger without requiring Raft or cluster services

### Requirement: Dogfood receipt CLI is tested
r[molten.dogfood.cli_tests] Molten SHOULD test the local dogfood receipt list, show, validate, export, Nix evidence export, and Nix evidence verification CLI paths.

#### Scenario: CLI test covers receipt readback
- GIVEN a CLI test runs local dogfood
- WHEN it lists, shows, validates, exports, and verifies dogfood receipts
- THEN the commands pass for current evidence and emit deny verification receipts for stale Nix refs

### Requirement: Dogfood receipt graph integrity is tested
r[molten.dogfood.property_tests] Molten SHOULD test dogfood receipt child graph integrity and redacted export stability with deterministic examples or property tests.

#### Scenario: Receipt graph remains stable
- GIVEN a dogfood report with child receipt refs and redaction-safe export
- WHEN tests recompute canonical refs and export the report
- THEN the exported artifact ref matches the original report ref and child receipt refs remain stable
