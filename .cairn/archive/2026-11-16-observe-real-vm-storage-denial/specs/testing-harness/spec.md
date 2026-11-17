## MODIFIED Requirements

### Requirement: VM storage and state-root faults are bounded
r[molten.testing.nixos_vm_fault_injection.storage_state_faults] Molten SHOULD execute bounded storage and state-root fault cases such as missing artifacts, permission-denied state roots, corrupted diagnostic-only logs, or bounded disk pressure where deterministic VM support permits. An executed-denial claim MUST be based on the actual existing protected node operation's typed pre-write filesystem failure under a non-root service identity with observed effective permissions, selected executable and source/config bindings, and independently captured matching protected state before and after the attempt. If these observations or capabilities are unavailable, Molten MUST classify the case unavailable rather than observed.

#### Scenario: Permission-denied state root fails before mutation
- GIVEN a VM node state root is made unwritable for a targeted operation
- WHEN the operation attempts to write control, ledger, or receipt state
- THEN Molten emits a denial or failure receipt before accepting side effects as pass evidence.

#### Scenario: Status health write is denied before mutation
- GIVEN a signed selected node has initialized and its service has stopped
- AND the selected state root is unwritable by the non-root service identity without DAC-bypass capabilities
- WHEN that identity executes the existing `node status` health write against the declared VM target
- THEN the OS denies the file open before any write with a typed permission error
- AND the command fails while a canonical outside-root operation artifact binds its actual executable, identity, target and failure phase
- AND independent pre/post state observations show the absent target and identical protected tree.

#### Scenario: Unsupported execution stays unavailable
- GIVEN the VM cannot ensure the intended identity, filesystem denial or independent observations
- WHEN the storage fault is reported
- THEN it remains unavailable with a reason, without an observed-denial status or synthetic child.

### Requirement: VM executable faults emit canonical receipts
r[molten.testing.nixos_vm_fault_injection.fault_receipts] Molten MUST emit canonical VM fault receipts that bind fault descriptor refs, host-support status, pre-fault refs, injection evidence refs, post-fault child refs, decisions, diagnostics, replay status, diagnostic log refs, and evidence-only caveats. Molten MUST validate an observed storage-denial receipt against actual canonical child and independent pre/post/permission artifact values. Matching references alone MUST NOT qualify executed denial; selected topology, operation, target, source/build, effective identity and permissions, typed pre-write EACCES and equal protected state MUST agree.

#### Scenario: Fault receipt binds pre and post evidence
- GIVEN an executable VM fault case completes
- WHEN the VM fault receipt is emitted
- THEN it identifies the fault descriptor, preflight evidence, injection evidence, observed child receipts, final decision, and any unavailable or degraded diagnostics.

#### Scenario: Arbitrary child cannot qualify denial
- GIVEN an authored fault receipt with a nonempty unrelated child ref or handwritten injection marker
- WHEN VM fault validation assesses an observed-denial claim
- THEN it denies that claim even if logs and caller-selected decision say denial.

#### Scenario: Real linked denial qualifies the bounded VM case
- GIVEN a receipt binds a typed actual status failure and independent configuration/pre/post observations for the selected VM node
- WHEN their canonical hashes and fields agree and no protected mutation occurred
- THEN fault validation accepts the bounded observed-denial claim without granting authority or external qualification.
