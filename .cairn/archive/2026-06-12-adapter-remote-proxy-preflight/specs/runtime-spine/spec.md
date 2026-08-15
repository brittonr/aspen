# Runtime Spine Delta: adapter and remote-proxy preflight

### Requirement: Adapter-backed actors require executable preflight
r[molten.runtime.adapter_remote_proxy_preflight.adapter] Adapter-backed actors MUST remain fail-closed until an adapter preflight receipt validates manifest refs, executable/artifact refs, ABI/schema refs, sandbox profiles, permission manifests, allowed hostcalls, and conformance refs.

#### Scenario: Missing adapter manifest is rejected
- GIVEN an adapter-backed actor without an adapter manifest/preflight receipt
- WHEN the harness attempts execution
- THEN execution is rejected before side effects occur

#### Scenario: Permission drift is rejected
- GIVEN an adapter manifest that declares one permission set
- WHEN the preflight receipt or execution request uses a different permission set
- THEN validation fails closed before the adapter can run

### Requirement: Remote proxies require endpoint and trust preflight
r[molten.runtime.adapter_remote_proxy_preflight.remote_proxy] Remote-proxy actors MUST remain fail-closed until a remote-proxy preflight receipt validates peer identity, endpoint/protocol refs, actor contract refs, attenuation/proof refs, transport profile, and trust requirements.

#### Scenario: Unknown peer cannot satisfy trusted gate
- GIVEN a remote-proxy actor reached through an unknown peer
- WHEN a gate profile requires trusted peer identity and signatures
- THEN transport may fetch bytes or diagnostics
- BUT pass evidence is rejected until trust evidence validates

#### Scenario: Endpoint contract mismatch is rejected
- GIVEN a remote endpoint advertises an actor contract ref
- WHEN the local registry expects a different contract ref
- THEN proxy execution fails closed before sending actor input

### Requirement: Adapter and proxy communication uses canonical envelopes
r[molten.runtime.adapter_remote_proxy_preflight.envelopes] Adapter and remote-proxy actors MUST exchange actor input, hostcall request/decision, actor output, effects, and transcripts as canonical Preserves envelopes.

#### Scenario: Adapter hostcall goes through admission
- GIVEN an adapter-backed actor requests a send operation
- WHEN the request reaches the runtime shell
- THEN it is represented as canonical hostcall evidence
- AND policy, capability, budget, and replay checks run before acceptance

#### Scenario: Malformed proxy output is rejected
- GIVEN a remote proxy returns bytes that do not decode to the expected canonical actor-output schema
- WHEN the runtime validates the response
- THEN execution fails closed before commit

### Requirement: Deterministic gates require replayable transcripts
r[molten.runtime.adapter_remote_proxy_preflight.replay] Adapter and remote-proxy actors MUST provide verified execution transcripts or effect logs before their reports can satisfy deterministic gates.

#### Scenario: Missing transcript is diagnostic only
- GIVEN a remote-proxy actor run without a replayable transcript
- WHEN the report is gated for deterministic pass evidence
- THEN the gate rejects the report as non-replayable

#### Scenario: Transcript mismatch is reported
- GIVEN a recorded adapter transcript whose hostcall decision differs from replay
- WHEN replay validates the report
- THEN replay reports an adapter/proxy transcript divergence

### Requirement: Process and transport success do not grant authority
r[molten.runtime.adapter_remote_proxy_preflight.authority] Starting an adapter process or connecting to a remote peer MUST NOT imply capability authority, signer trust, policy acceptance, or gate acceptance.

#### Scenario: Undeclared ambient network attempt fails closed
- GIVEN an adapter process attempts network access not declared in its permission manifest
- WHEN the sandbox/preflight detects the request
- THEN execution fails closed and records executor-boundary evidence

#### Scenario: Remote transport succeeds but capability is missing
- GIVEN a remote proxy connection succeeds
- WHEN the remote actor requests an operation without local capability authority
- THEN the local runtime denies the hostcall and records the denial evidence
