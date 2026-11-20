## ADDED Requirements

### Requirement: Replica configuration is exact and bounded
r[molten.fabric_consistency.replica_shell.config] The shell MUST admit one
bounded canonical configuration binding group, node, membership, peer
endpoints, listener key/address, durability root, application manifest and
handler, protocol/timer/entropy/supervision references, and finite limits. It
MUST reject test references, workspace-relative paths, missing or duplicate
peers, unknown fields, ambient key discovery, and unsupported address scopes.

#### Scenario: Exact configuration starts
- GIVEN one complete canonical configuration and a readable listener key
- WHEN the replica command admits it
- THEN it binds only the declared address and peers and starts the declared
  group/node.

#### Scenario: Test or relative input is supplied
- GIVEN a configuration containing a test reference or relative durability
  path
- WHEN admission runs
- THEN the command denies before binding a listener or creating state.

### Requirement: Fresh and recover lifecycle are distinct
r[molten.fabric_consistency.replica_shell.lifecycle] The shell MUST implement
distinct fresh and recover modes. Fresh MUST create a new durability root.
Recover MUST reopen the exact existing root and run the existing recovery path.
Readiness MUST be published only after listener bind, service start, and
application admission. Stop MUST be bounded and MUST emit a terminal receipt.

#### Scenario: Recovered replica publishes recovery evidence
- GIVEN an existing durability root from the same exact configuration
- WHEN the replica starts in recover mode
- THEN it runs recovery, publishes its recovery ref, and reaches ready without
  asserting machine-loss durability.

#### Scenario: Wrong mode is denied
- GIVEN a fresh request for an existing root or a recover request for an absent
  root
- WHEN the command starts
- THEN it denies without overwriting or inventing state.

### Requirement: Ingress and event processing stay bounded
r[molten.fabric_consistency.replica_shell.ingress] The shell MUST use the
existing admitted ingress pump with explicit event capacity, delivery
rate/window, accept timeout, and poll interval. It MUST NOT add an unbounded
queue, unbounded read, ambient listener, or implicit retry.

#### Scenario: Stop drains the owned listener
- GIVEN a ready replica and an explicit stop
- WHEN the event loop exits
- THEN the ingress pump is shut down, the owned listener is closed, and the
  terminal receipt reports clean shutdown.

### Requirement: Application binding is closed
r[molten.fabric_consistency.replica_shell.application] The shell MUST bind one
supported admitted application manifest and handler. The first supported
production handler is the collaboration-scope application. Unknown, test-only,
or mismatched handlers MUST fail closed before readiness.

#### Scenario: Unsupported handler is denied
- GIVEN a configuration naming an unadmitted or test-only application handler
- WHEN admission runs
- THEN the shell denies before starting the consensus service.

### Requirement: Evidence is scoped
r[molten.fabric_consistency.replica_shell.evidence] Startup and terminal
receipts MUST bind the exact group, node, source/build/closure refs, peer
endpoints, durability root, recovery ref where present, readiness, and clean
shutdown. Status readback MUST be a projection and MUST NOT grant authority,
policy, resource, deployment, machine-loss, or release eligibility.

#### Scenario: Process exit alone is not durability evidence
- GIVEN a clean terminal receipt
- WHEN a consumer reads it
- THEN machine-loss durability, guest execution, and release eligibility remain
  explicit non-claims.

### Requirement: Guest claims remain separate
r[molten.fabric_consistency.replica_shell.nonclaims] A native replica run MUST
NOT be represented as guest, KVM, multi-VM, ChaosControl campaign, production,
or release evidence. Guest packaging, device mapping, and campaign execution
remain external owner responsibilities.

#### Scenario: Native receipt is promoted
- GIVEN a native three-process receipt
- WHEN a consumer labels it as guest or campaign qualification
- THEN the promotion is rejected or classified as an overclaim.

### Requirement: Validation covers success and denial
r[molten.fabric_consistency.replica_shell.validation] Validation MUST include a
native three-process election/commit/read/snapshot/restart/stale-fence run
through the production command, plus negative configuration, mode, listener,
application, and shutdown cases.

#### Scenario: Native three-process run passes
- GIVEN three exact configurations and keys
- WHEN the production command runs the bounded scenario
- THEN readiness, commit, read, snapshot catch-up, restart, and stale fencing
  are observed and the processes exit cleanly.
