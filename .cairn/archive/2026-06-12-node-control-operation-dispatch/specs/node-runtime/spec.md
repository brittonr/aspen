# Node Runtime Delta: Control Operation Dispatch

### Requirement: Install operation dispatch is ledger-backed
r[molten.node_control_operation.spec.install_dispatch] A node control `install` request MUST resolve its payload from the node ledger, install through the node artifact registry, and bind artifact install evidence in the final control receipt.

#### Scenario: Install request passes
- GIVEN a running node, explicit control authority/policy/resource refs, and a payload value imported into the node ledger
- WHEN the `install` request is submitted and dispatched
- THEN the payload is installed into the node registry
- AND the final node control receipt binds the artifact install receipt.

### Requirement: Run operation dispatch is target-local
r[molten.node_control_operation.spec.run_dispatch] A node control `run` request MUST execute only a ledger-resolved job execution request and matching admission receipt against node-local registry, storage, cache, and chunk roots.

#### Scenario: Run request passes
- GIVEN a running node with a synced and admitted job in the node registry
- WHEN a `run` request references the execution request and matching admission receipt
- THEN dispatch emits a job execution receipt
- AND the final control receipt binds that job subreceipt.

### Requirement: Gate operation dispatch validates strict evidence
r[molten.node_control_operation.spec.gate_dispatch] A node control `gate` request MUST validate the ledger-resolved gate receipt for the requested subject before passing.

#### Scenario: Gate request passes
- GIVEN a running node, a target subject ref, and a strict clean Octet gate receipt imported into the node ledger
- WHEN the `gate` request is dispatched
- THEN an Octet source-gate validation receipt is emitted
- AND the final control receipt binds the validation receipt.

### Requirement: Side-effecting operations fail closed before effects
r[molten.node_control_operation.spec.fail_closed_before_effects] Side-effecting control operations MUST deny before operation side effects when request authority, policy, resource, payload, target, or ledger evidence is missing or malformed.

#### Scenario: Missing payload denies before install
- GIVEN a running node and an `install` request without a payload ref
- WHEN the request is dispatched
- THEN no artifact is installed
- AND a deny operation receipt and deny control receipt are written.

### Requirement: Operation receipts are imported
r[molten.node_control_operation.spec.ledger_imports] Control operation dispatch MUST import operation subreceipts, successful installed artifact records, job execution receipts, gate validation receipts, and final control receipts into the node ledger.

#### Scenario: Operation evidence is catalogable
- GIVEN a dispatched `install`, `run`, or `gate` request
- WHEN the node ledger is listed
- THEN the corresponding operation subreceipt and final node control receipt artifacts are present.

### Requirement: Operation dispatch tests cover pass and deny
r[molten.node_control_operation.spec.tests] The implementation MUST include library or CLI coverage for passing install, run, and gate dispatch, plus fail-closed missing evidence behavior.

#### Scenario: Operation dispatch tests pass
- GIVEN the Molten test suite
- WHEN node control operation dispatch tests run
- THEN install, run, gate, and denial paths are covered by parseable canonical receipts.
