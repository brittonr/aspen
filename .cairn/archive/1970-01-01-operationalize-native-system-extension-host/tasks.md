# Tasks: Operationalize the native system-extension host

## Phase 1: Native profile and callback contract

- [x] [depends:add-bounded-execution-fabric-port] Consume the accepted bounded execution port without adding a second process shell. r[molten.system_extension.native_host.execution]
- [x] [serial] Define the native-process profile, executable admission, callback envelope, callback outcome, status, and non-claim schemas. r[molten.system_extension.native_host.profile] r[molten.system_extension.native_host.callback_protocol] r[molten.system_extension.native_host.executable]
- [x] [parallel] Add typed Nickel profiles and positive and negative fixtures for executable cohorts, callback bounds, ports, authority, resources, and lifecycle. r[molten.system_extension.native_host.profile] r[molten.system_extension.native_host.nonclaims]
- [x] [parallel] Publish independent callback producer and consumer fixtures for one conforming external executable. r[molten.system_extension.native_host.callback_protocol] r[molten.system_extension.native_host.validation]

## Phase 2: Pure service and recovery decisions

- [x] [serial] Add pure executable, instance, callback, effect, generation, lifecycle, ingress, recovery, drain, and removal admission over explicit facts. r[molten.system_extension.native_host.executable] r[molten.system_extension.native_host.recovery]
- [x] [serial] Define durable instance, callback-intent, effect-intent, checkpoint, unresolved-operation, and terminal-state records. r[molten.system_extension.native_host.durability] r[molten.system_extension.native_host.intent]
- [x] [parallel] Add positive and negative pure tests for valid transitions, stale generations, duplicates, unknown outcomes, incompatible state, and removal blockers. r[molten.system_extension.native_host.effect_completion] r[molten.system_extension.native_host.validation]

## Phase 3: Native executor and effect routing

- [x] [serial] Implement `NativeProcessSystemExtensionExecutor` with one bounded process per callback and a cleared environment. r[molten.system_extension.native_host.execution]
- [x] [serial] Parse and admit callback output before committing state or releasing effects. r[molten.system_extension.native_host.callback_protocol] r[molten.system_extension.native_host.effects]
- [x] [serial] Persist callback intent before process start and effect intent before exact fabric-port routing. r[molten.system_extension.native_host.intent]
- [x] [serial] Route effect completions back through generation-fenced callback events without inferring workload success. r[molten.system_extension.native_host.effect_completion]
- [x] [parallel] Add executor tests for valid bytes, malformed output, output flood, timeout, cancellation, nonzero exit, teardown failure, and unknown completion. r[molten.system_extension.native_host.execution] r[molten.system_extension.native_host.validation]

## Phase 4: Node service and operator workflows

- [x] [serial] Add the durable extension-instance registry and node composition without workload-specific branches. r[molten.system_extension.native_host.durability] r[molten.system_extension.native_host.neutrality]
- [x] [serial] Add versioned service ingress with exact transport, ALPN, framing, acknowledgement, authority, policy, resource, and generation bindings. r[molten.system_extension.native_host.ingress]
- [x] [serial] Add install, start, request, status, recover, drain, stop, and remove operations with canonical receipts. r[molten.system_extension.native_host.operator]
- [x] [serial] Implement startup inventory and explicit recovery for not-started, running-observed, terminal, unknown, and stale operations. r[molten.system_extension.native_host.recovery]
- [x] [parallel] Add negative operator tests for stale manifests, incompatible checkpoints, missing authority, unresolved work, incomplete teardown, and hidden fallback. r[molten.system_extension.native_host.operator] r[molten.system_extension.native_host.validation]

## Phase 5: Conformance and closeout

- [x] [serial] Run a parent-observed separate-process fixture through install, activate, request, effect, checkpoint, restart, recover, drain, and stop. r[molten.system_extension.native_host.validation]
- [x] [parallel] Add offline artifact-index verification and tamper tests for executable, callback, state, effect, checkpoint, lifecycle, and parent-child evidence. r[molten.system_extension.native_host.validation]
- [x] [serial] Add architecture checks for direct process calls, workload-name branches, raw handles, mutable sibling dependencies, and profile fallback. r[molten.system_extension.native_host.neutrality]
- [x] [serial] Document the callback protocol, deployment, recovery, operator workflows, claim level, and all non-claims. r[molten.system_extension.native_host.nonclaims]
- [x] [serial] Run formatting, focused and workspace tests, Clippy, Octet, Nickel, Cairn validation and gates, traceability, and relevant Nix checks. r[molten.system_extension.native_host.validation] r[molten.system_extension.native_host.nonclaims]

## Verification Coverage

- `Scenario: Native execution is unavailable` -> profile fallback negative fixture
- `Scenario: Child returns malformed or extra output` -> callback parser negative tests
- `Scenario: Path or artifact possession is the only evidence` -> executable admission negative test
- `Scenario: Callback times out or floods output` -> bounded executor tests
- `Scenario: Durable state is missing or incompatible` -> startup quarantine tests
- `Scenario: Effect routing loses acknowledgement` -> unresolved-effect recovery test
- `Scenario: Callback requests an unbound or stale effect` -> effect admission tests
- `Scenario: Completion is stale or duplicated` -> completion fencing tests
- `Scenario: Transport accepts but callback admission fails` -> ingress denial test
- `Scenario: Effect outcome is unknown` -> reconciliation test
- `Scenario: Removal has unresolved work` -> operator removal negative test
- `Scenario: Node core switches on a workload name` -> architecture check
- `Scenario: Separate-process service fixture passes` -> parent harness
- `Scenario: Local pilot service succeeds` -> scoped status and documentation
