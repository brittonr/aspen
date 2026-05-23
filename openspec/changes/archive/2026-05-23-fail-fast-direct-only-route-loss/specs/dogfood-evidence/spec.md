## MODIFIED Requirements

### Requirement: VM-CI dogfood route-loss evidence [r[dogfood-evidence.vmci.direct-route-loss]]

VM-CI dogfood diagnostics MUST classify host-client direct route loss separately from node health failure, guest worker readiness, and post-registration CI execution stalls.

#### Scenario: Route loss after pipeline discovery is classified [r[dogfood-evidence.vmci.direct-route-loss.after-pipeline-discovery]]

- GIVEN a `dogfood-local-vmci -- full` run has reached node health, cluster initialization, Forge source push, and CI pipeline discovery
- AND subsequent CI wait RPCs repeatedly fail because the client has no address lookup or direct route source for the host node peer
- WHEN the run exits or saves diagnostic evidence
- THEN the evidence MUST report the highest reached boundary as CI pipeline discovery
- AND it MUST classify the terminal failure as host-client direct route loss
- AND it MUST NOT misclassify the run as VM worker execution, workspace/blob materialization, or generic CI timeout

#### Scenario: Route-loss evidence is redacted and bounded [r[dogfood-evidence.vmci.direct-route-loss.redacted]]

- GIVEN dogfood records a direct-route-loss failure
- WHEN the failure is rendered in logs, receipts, or diagnosis output
- THEN the output MUST include a bounded peer identifier, relay/discovery policy summary, and direct-address count or absence
- AND it MUST redact tickets, secret keys, cookies, tokens, and long opaque credential-like values
