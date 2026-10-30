## ADDED Requirements

### Requirement: DAG synchronization exposes owner-defined fabric admission
r[molten.dag_sync.fabric_admission] The DAG synchronization owner MUST export a canonical fabric descriptor with exact version, request/receipt schema identities, reviewed implementation/conformance references, required authority/resources and explicit non-claims. Consumers MUST resolve it through normal fabric and system-extension admission and MUST NOT substitute fixture references.

#### Scenario: Admitted receiver effect executes
- GIVEN a canonical owner-defined DAG binding and a current host-owned successful callback
- WHEN the dispatcher routes its exact request
- THEN the existing DAG shell obtains authority/resources and performs receiver-owned transport/content verification before publishing completion.

#### Scenario: Binding does not imply import authority
- GIVEN a successful DAG binding or complete graph receipt
- WHEN local import gates have not passed
- THEN trusted registries and domain heads MUST remain unchanged.
