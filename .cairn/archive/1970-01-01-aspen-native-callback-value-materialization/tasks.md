# Tasks: Materialize native callback values

## Phase 1: Protocol and pure admission

- [x] [depends:operationalize-native-system-extension-host] Define the exact v2 host, envelope, outcome, ALPN, and framing cohort without fallback. r[molten.system_extension.native_host.value_protocol]
- [x] [serial] Add bounded reference-and-byte value types and pure identity admission. r[molten.system_extension.native_host.value_materialization] r[molten.system_extension.native_host.value_publication]
- [x] [parallel] Add positive and negative wire fixtures for exact, missing, corrupt, oversized, legacy, and mixed-version values. r[molten.system_extension.native_host.value_validation]

## Phase 2: Value port and durable state

- [x] [serial] Add the narrow materialize and publish value port with an in-memory conformance adapter. r[molten.system_extension.native_host.value_materialization] r[molten.system_extension.native_host.value_publication]
- [x] [serial] Persist semantic `state_ref` separately from lifecycle `checkpoint_ref`. r[molten.system_extension.native_host.semantic_state]
- [x] [serial] Add durable publication operation intent, terminal, and unknown classifications. r[molten.system_extension.native_host.value_intent]
- [x] [parallel] Add positive and negative journal and recovery tests for semantic state and unresolved publications. r[molten.system_extension.native_host.semantic_state] r[molten.system_extension.native_host.value_validation]

## Phase 3: Executor and service ordering

- [x] [serial] Commit callback intent before materialization and process execution. r[molten.system_extension.native_host.value_intent]
- [x] [serial] Materialize ingress, prior state, completions, and checkpoints into the v2 callback envelope. r[molten.system_extension.native_host.value_materialization]
- [x] [serial] Admit, intent, and publish output, effect-request, next-state, and checkpoint values before projection. r[molten.system_extension.native_host.value_publication]
- [x] [serial] Block state replacement and provider routing on rejected or uncertain publication. r[molten.system_extension.native_host.value_intent]
- [x] [parallel] Add executor and service tests for exact ordering, missing input, bad identity, process failure, publication rejection, and publication uncertainty. r[molten.system_extension.native_host.value_validation]

## Phase 4: Profiles, fixtures, and closeout

- [x] [serial] Update typed native-host profiles and negative fixtures for v2 value bounds and no fallback. r[molten.system_extension.native_host.value_protocol]
- [x] [serial] Run a separate-process fixture through ingress, state, effect body, checkpoint, restart, and recovery. r[molten.system_extension.native_host.value_validation]
- [x] [parallel] Add architecture checks that keep value I/O in the shell and workload semantics outside Aspen. r[molten.system_extension.native_host.value_validation]
- [x] [serial] Document protocol v2, operation ordering, recovery, deployment adapter duties, and non-claims. r[molten.system_extension.native_host.value_protocol]
- [x] [serial] Run formatting, focused and workspace tests, Clippy, Octet, Nickel, Cairn validation and gates, traceability, and relevant Nix checks. r[molten.system_extension.native_host.value_validation]

## Verification Coverage

- `Scenario: Materialized callback values match their references` -> v2 wire and executor positive tests
- `Scenario: A required value is missing or corrupt` -> missing and corruption tests
- `Scenario: Returned values publish successfully` -> publication and provider-order test
- `Scenario: Returned bytes are absent or substituted` -> reference-only and substitution tests
- `Scenario: Publication fails before acceptance` -> terminal publication test
- `Scenario: Publication acceptance is uncertain` -> unknown publication and recovery test
- `Scenario: Request updates semantic state` -> consecutive callback state test
- `Scenario: Restart observes unresolved value work` -> durable recovery inventory test
- `Scenario: Version two is selected` -> exact profile fixture
- `Scenario: Legacy or mixed protocol is supplied` -> negative profile and wire fixtures
- `Scenario: Separate-process materialization passes` -> parent-child host fixture
- `Scenario: Negative evidence is absent` -> closeout task gate
