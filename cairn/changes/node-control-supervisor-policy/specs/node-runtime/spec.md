# Node Runtime Delta: Supervisor Policy

### Requirement: Supervisor policies are canonical artifacts
r[molten.node_control_supervisor_policy.spec.policy_artifact] Node-control supervisor policy MUST be represented by canonical `node-control-supervisor-policy-v1` artifacts that bind max restarts, restart window ticks, heartbeat timeout ticks, shutdown drain ticks, stale-lock recovery mode, policy refs, evidence refs, and lifecycle checks.

#### Scenario: Policy fixture is importable
- GIVEN an initialized node
- WHEN a supervisor policy fixture is written with `--state-root`
- THEN the policy has a stable artifact ref
- AND it is imported into the node ledger.

### Requirement: Supervisor receipts are canonical
r[molten.node_control_supervisor_policy.spec.supervisor_receipts] Node-control supervisor lifecycle decisions MUST be represented by canonical `node-control-supervisor-receipt-v1` receipts and service-run receipts MUST bind the governing supervisor policy and supervisor receipt refs when a policy is used.

#### Scenario: Policy-governed serve records supervisor refs
- GIVEN a running node and a supervisor policy artifact
- WHEN `molten node serve --supervisor-policy` runs
- THEN the service-run receipt records the policy ref
- AND it records at least one supervisor receipt ref.

### Requirement: Supervisor policy CLI is available
r[molten.node_control_supervisor_policy.spec.policy_cli] The CLI MUST expose a supervisor policy fixture helper and a `serve --supervisor-policy` input for policy-governed service runs.

#### Scenario: CLI serves with policy
- GIVEN a running node and supervisor policy file
- WHEN `molten node serve --supervisor-policy <path>` is executed
- THEN the command emits a canonical service-run receipt
- AND the receipt binds supervisor policy evidence.

### Requirement: Stale service locks fail closed without policy recovery
r[molten.node_control_supervisor_policy.spec.stale_lock_gate] Node-control service runners MUST deny duplicate or stale service locks before side effects unless an imported supervisor policy explicitly allows stale-lock recovery.

#### Scenario: Policy admits stale lock recovery
- GIVEN a running node with an existing service lock
- AND a supervisor policy that allows stale-lock recovery
- WHEN service serve starts
- THEN a stale-lock recovery supervisor receipt is written
- AND a new service run may proceed.

### Requirement: Restart attempts are bounded
r[molten.node_control_supervisor_policy.spec.restart_bounds] Policy-governed service runners MUST enforce bounded restart attempts before taking a service lock and MUST emit supervisor receipts for admitted or denied restart attempts.

#### Scenario: Restart bound is exceeded
- GIVEN a supervisor policy with a bounded restart count
- AND prior service-run receipts already exceed that count
- WHEN service serve starts again with that policy
- THEN the service run is denied before a service lock is taken
- AND a restart-attempt denial supervisor receipt is recorded.

### Requirement: Shutdown drain is bounded
r[molten.node_control_supervisor_policy.spec.shutdown_drain] Policy-governed service runners MUST emit shutdown-drain supervisor receipts and MUST deny service runs whose observed shutdown drain exceeds the policy bound.

#### Scenario: Shutdown drain exceeds policy
- GIVEN a supervisor policy with a tight shutdown drain bound
- AND a shutdown request in the node-control inbox
- WHEN service serve processes the shutdown
- THEN the service run stops the node
- BUT the service-run decision is deny with shutdown-drain diagnostics.

### Requirement: Supervisor policy is not operation authority
r[molten.node_control_supervisor_policy.spec.not_authority] Supervisor policy artifacts and supervisor receipts MUST NOT satisfy node-control operation authority, peer bootstrap, resource policy, delivery idempotency, or payload provenance requirements.

#### Scenario: Supervisor policy does not authorize operations
- GIVEN a policy-governed service runner
- AND an ingress envelope without admitted authority
- WHEN the envelope reaches durable ingress delivery
- THEN supervisor policy evidence does not satisfy authority
- AND enqueue still denies before operation side effects.

### Requirement: Supervisor gates fail closed
r[molten.node_control_supervisor_policy.spec.fail_closed] Supervisor lifecycle gates MUST fail closed for malformed policy bounds, unknown policy artifacts, stale locks without recovery, duplicate active runners, restart-bound violations, and over-bound shutdown drain.

#### Scenario: Duplicate runner is denied
- GIVEN an active service lock
- WHEN a second policy-governed service runner starts
- THEN it emits a duplicate-runner denial receipt
- AND it does not dispatch pending control requests.
