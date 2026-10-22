# System Extension Runtime Delta

## ADDED Requirements

### Requirement: Runtime-local work binds to the executing incarnation
r[molten.system_extension.incarnation_fencing] Aspen MUST distinguish the service key, the implementation generation, and the runtime incarnation of an executing instance. Every runtime-local timer, callback completion, and failure notification MUST bind the incarnation that produced it, and dispatch MUST reject work whose incarnation does not match the executing instance, including after a restart that preserves the generation. Restart paths MUST advance the fencing identity before old runtime-local work can act on the replacement instance.

#### Scenario: Delayed timer after same-generation restart
- GIVEN instance A scheduled a timer and then restarted as instance B without changing the implementation generation
- WHEN the timer fires for instance A after B is running
- THEN dispatch rejects the timer delivery with a typed stale-instance issue and instance B state does not change.

#### Scenario: Callback completion from the previous instance
- GIVEN instance A issued a callback whose completion is still pending and then restarted as instance B
- WHEN the completion arrives after B is running
- THEN the completion is rejected as stale and no effect, state delta, or externally visible output from it is planned.

#### Scenario: Conformance trace guards every restart path
- GIVEN the permanent restart-fencing conformance trace in the deterministic test suite
- WHEN any restart path is added or changed such that old runtime-local work can act on the replacement
- THEN the trace fails and names the accepted delivery.

### Requirement: Durable work survives restart only through explicit readmission
r[molten.system_extension.durable_readmission] Durable logical work addressed to a service MAY survive restart, but it MUST be re-admitted under the replacement instance with a current delivery claim before it can act. Runtime-local work MUST NOT be presented as durable work, and durable work MUST NOT be treated as a still-valid callback of a previous instance.

#### Scenario: Durable message readmitted after restart
- GIVEN a durable message addressed to the service exists when the instance restarts
- WHEN the replacement instance starts and the shell presents the message with a fresh delivery claim bound to the current incarnation
- THEN the message is processed under the replacement instance and the claim records the current incarnation.

#### Scenario: Old callback presented as durable work
- GIVEN instance A's callback completion exists when instance B is running
- WHEN that completion is presented through the durable readmission path
- THEN readmission rejects it because its incarnation predates the current claim.
