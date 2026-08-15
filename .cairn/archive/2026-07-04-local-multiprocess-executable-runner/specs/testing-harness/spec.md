## ADDED Requirements

### Requirement: Local multiprocess runner starts real node processes
r[molten.testing.multinode.local_multiprocess_executable_runner] Molten SHOULD provide an executable local multiprocess runner that consumes a validated process plan, starts isolated `molten node` processes, runs a bounded cross-process workflow, and emits canonical startup, workflow, shutdown, cleanup, and run receipts.

#### Scenario: Local runner records cross-process evidence
- GIVEN a valid local multiprocess plan with isolated node ids, state-root handles, transport handles, expected receipt refs, and cleanup policy
- WHEN the runner starts the node processes and executes the workflow
- THEN the run receipt binds the plan ref, startup refs, workflow refs, shutdown refs, cleanup refs, diagnostics, and evidence-only caveats
- AND the receipt states that local multiprocess evidence is not VM evidence.

#### Scenario: Runner remains a thin shell over the pure plan
- GIVEN a local multiprocess execution request
- WHEN the runner prepares and executes the workflow
- THEN process spawning, filesystem writes, signal handling, and cleanup stay in the shell
- AND planning, receipt classification, and pass/deny decisions are testable as pure functions.

### Requirement: Local multiprocess runner fails closed on lifecycle and cleanup errors
r[molten.testing.multinode.local_multiprocess_runner_negatives] Molten MUST reject local multiprocess pass evidence when tickets are stale, state roots collide, transport handles collide, required receipts are missing, child processes orphan, timeouts occur, or cleanup evidence is absent.

#### Scenario: Missing workflow receipt denies pass evidence
- GIVEN a local multiprocess run where a node starts but the required workflow receipt is missing
- WHEN the run receipt is built
- THEN the decision denies before pass evidence is accepted
- AND diagnostics name the missing workflow receipt.

#### Scenario: Orphaned child process blocks pass
- GIVEN a local multiprocess run whose child process remains alive after cleanup
- WHEN cleanup validation runs
- THEN cleanup evidence records denial
- AND the final run receipt cannot pass.
