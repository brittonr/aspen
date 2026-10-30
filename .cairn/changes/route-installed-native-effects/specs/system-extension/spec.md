# System Extension Installed Native Effects Delta

## ADDED Requirements

### Requirement: Explicit bounded installed native effect composition
r[molten.system_extension.native_effect_installation] The public native node host MUST install collaboration effect routing only from an explicitly selected bounded canonical configuration and actual admitted Running native supervisor. Configuration MUST bind the exact service, manifest, port binding/key/operation, scope group, active generation, policy, resource, installation and measured owned runtime artifact. Installation MUST reject mismatches before creating scope effect resources. Configuration possession and source verification MUST NOT grant effect authority or implicitly activate a native service. Unconfigured effects MUST remain unrouted and unresolved.

#### Scenario: Missing or mismatched installation
- GIVEN an actual admitted native callback whose outcome contains an approved collaboration effect
- WHEN configuration is absent, names another service/port/group/generation/policy/resource/installation, or its runtime artifact bytes differ
- THEN no mismatched scope provider is invoked, no service is implicitly activated, and the work is not reported complete.

#### Scenario: Current exact active installation
- GIVEN a Running actual native service and canonical runtime artifact with the exact admitted scope group and bindings
- WHEN current installation authority succeeds and pure runtime admission derives matching installation/group/policy/resource
- THEN only that concrete bounded runtime may be opened and registered for that member.

### Requirement: Independent current operator and scope effect authority
r[molten.system_extension.native_effect_authority] Effect resource installation and routing MUST require actual current verifier-backed operator authority for their exact bundle/configuration/effect bindings, including service, port, group, generation/incarnation, policy, resource, schemas, request, byte accounting and logical tick. Scope mutation MUST additionally require the owning ScopeApplication's fresh quorum/currentness observation and independent current policy/UCAN/Basalt/revocation admission immediately before queueing the protected command. Source trust, matching references, historical staged Decisions and successful callbacks MUST NOT substitute for these authorities. Replay reservations MUST survive actual process replacement; secrets MUST NOT appear in status or receipt projections.

#### Scenario: Denied or substituted authority
- GIVEN an otherwise structurally valid effect runtime and callback
- WHEN a required current operator/scope grant is absent, expired, revoked, replayed or bound to another request/configuration/port/generation
- THEN the protected command is not admitted merely because source verification or a prior staged snapshot succeeded.

#### Scenario: Historical staged command is not current permission
- GIVEN a previously staged command and historical authorization receipts
- WHEN current scope state, required policy, revocation or quorum/currentness no longer admits that command
- THEN routing does not enqueue it under the historical decision alone.

### Requirement: Actual committed application state and native completion
r[molten.system_extension.native_effect_completion] NativeRuntime MUST route approved effects through the existing Supervisor intent-recording route mechanism and deliver exact verified completions through its existing materialized generation-fenced completion callback. The concrete scope runtime MUST observe the actual active Raft replica apply the exact committed command to ScopeApplication before returning a committed result. Queue and committed-result schemas MUST remain semantically distinct. Root logical work MUST become complete only after every approved effect has a real successful completion callback without further unhandled effects; implementations MUST NOT clear approved effects or fabricate callback success to imply completion.

#### Scenario: Proposal queued without commit
- GIVEN a genuine native callback whose protected proposal enters the actual bounded replica queue
- WHEN no actual matching committed application transition is observed
- THEN queue acceptance is not a committed effect result or completed native work.

#### Scenario: Actual proposal, commit and completion
- GIVEN an exactly installed and currently authorized collaboration effect
- WHEN the actual replica commits its exact staged command and ScopeApplication applies it
- THEN the materialized committed result is delivered through the real native completion callback and only verified successful completion permits work completion.

#### Scenario: Completion produces further effects
- GIVEN a committed protected effect and its actual native completion callback
- WHEN that callback fails or emits additional effects without separately admitted handling
- THEN the parent work is not falsely reported complete.

### Requirement: Durable effect uncertainty without implicit retries
r[molten.system_extension.native_effect_uncertainty] Native effect intent, proposal acceptance, committed application state and completion callback MUST remain distinct durable boundaries. Definite denial MUST NOT be represented as protected-resource success. Once an effect may have occurred, provider, storage, process-loss or completion failure MUST leave operator-visible Unknown/reconciliation-required work rather than fabricated success or automatic retry. Reopening MUST observe the owning native/application journals and MUST NOT requeue an uncertain predecessor effect merely because configuration still matches.

#### Scenario: Failure after possible effect
- GIVEN persisted native effect intent and a proposal that may have reached the active replica
- WHEN the provider result, committed-state observation or native completion callback is lost or fails
- THEN work remains unresolved and no callback/proposal is automatically repeated as a recovery shortcut.

#### Scenario: Process replacement with unresolved predecessor
- GIVEN a native journal retaining an unresolved predecessor effect
- WHEN a replacement node loads the same canonical configuration
- THEN the router preserves the predecessor fence and requires explicit owner-governed reconciliation instead of reusing configuration as retry permission.

### Requirement: Actual retained runtime and resource ownership
r[molten.system_extension.native_effect_lifetime] The actual public node serve/run-loop MUST retain the concrete admitted native router and actual scope replica/application/resources for the lifetime of its owning InstalledHost. Runtime construction MUST wait for real bounded worker startup success before hosted readiness. Required admission failures MUST prevent readiness. Shutdown/error cleanup MUST close only the current owner's resources; no ambient fallback, spawn-and-assume-ready, implicit activation or unbounded retry loop is permitted.

#### Scenario: Serving node owns the actual router
- GIVEN a successful exactly authorized installed runtime
- WHEN the real node enters its serving/listener loop
- THEN its actual host retains the scope runtime through that loop and readiness comes only from successful current ownership, not a stale marker or unrelated process.

#### Scenario: Worker startup or ownership fails
- GIVEN selected effect configuration
- WHEN actual replica/storage/transport/worker startup fails or a required admission is denied
- THEN no successful installed-effect readiness is published and only acquired owned resources are released.

### Requirement: Genuine installed collaboration consumer proof
r[molten.system_extension.native_effect_consumer] The current installed collaboration-scope consumer MUST be exercised using a genuine materialized callback executable, actual independent test authority, the real public host, actual active Raft/Application machinery and actual materialized completion callback. Acceptance MUST observe the proposal, committed scope-state transition and native completion rather than infer them from callback output, queue length, fixture echoes or model state. Missing/stale/denied/uncommitted/post-effect-failure cases MUST remain fail-closed. Source implementation and task status MUST NOT be presented as recorded execution proof.

#### Scenario: Installed host end-to-end consumer
- GIVEN the parent-provisioned genuine signed source/admission cohort and a real installed native collaboration service
- WHEN the public host delivers currently authorized logical work through the admitted runtime
- THEN canonical observations identify the real callback, exact actual proposal, committed scope-state transition and successful real native completion callback.

#### Scenario: Missing external cohort or unrun validation
- GIVEN source artifacts but no observed integrated consumer run
- WHEN completion status or evidence is reported
- THEN validation remains open and no fabricated source receipt, fixture fallback or implied executed acceptance is supplied.
