# Runtime Spine Delta: Basalt/UCAN authority hardening

### Requirement: Runtime Basalt requests require verified UCAN authority
r[molten.runtime_spine.basalt_ucan_verified_authority] Runtime Basalt request admission MUST evaluate a pure Basalt/UCAN authority input containing contract id, resource, ability, request ref, verified grant refs, UCAN verification receipt refs, and Basalt policy refs, and MUST deny when any binding is missing, stale, malformed, or inconsistent.

#### Scenario: Verified authority admits matching runtime request
- GIVEN a runtime request with a Basalt contract id, resource, ability, verified UCAN grants, UCAN verification receipt refs, and matching Basalt policy refs
- WHEN the runtime authority core evaluates the request
- THEN the decision passes only for the exact bound contract id, resource, ability, and request ref
- AND the runtime may proceed to the next subsystem-specific gate.

#### Scenario: Bare UCAN ref is insufficient
- GIVEN a runtime request that carries only a canonical `ucan_ref` without verified grant refs and UCAN verification receipt refs
- WHEN Basalt request admission evaluates it
- THEN the decision is deny
- AND diagnostics state that possession of a content ref is not current authority.

### Requirement: Basalt/UCAN denials are traceable before side effects
r[molten.runtime_spine.basalt_ucan_trace_denials] Runtime and harness execution MUST emit canonical denial evidence when Basalt/UCAN authority validation denies for missing verification, wrong holder or session, wrong resource, wrong ability, revoked proof, replay denial, caveat denial, or Basalt policy denial, and MUST do so before committing messages, assertions, hostcalls, or other side effects.

#### Scenario: Revoked proof rolls back pending work
- GIVEN an actor turn stages a side effect and presents a UCAN proofset whose proof chain is revoked
- WHEN Basalt/UCAN authority validation runs
- THEN the runtime emits a deny receipt naming the revocation evidence
- AND the pending side effect is rolled back before commit.

#### Scenario: Basalt policy denial is evidence-bound
- GIVEN UCAN verification succeeds but the Basalt contract policy does not permit the requested resource or ability
- WHEN the runtime evaluates authority
- THEN the runtime emits a deny receipt bound to the Basalt enforcement receipt
- AND no side effect is admitted.
