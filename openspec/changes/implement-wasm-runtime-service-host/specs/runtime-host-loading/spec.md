## ADDED Requirements

### Requirement: WASM Runtime Service Host [r[runtime-host-loading.wasm-service-host]]
Aspen MUST provide a bounded WASM host contract for deterministic hooks, policies, plugins, and compatible runtime service fragments.

#### Scenario: WASM admission validates ABI and limits [r[runtime-host-loading.wasm-service-host.admission]]
- GIVEN a runtime declaration references a WASM module
- WHEN the WASM host admits the module
- THEN it SHALL verify module content identity, ABI version, entrypoint, fuel, memory, timeout, and capability policy before instantiation

#### Scenario: Host functions are capability-scoped [r[runtime-host-loading.wasm-service-host.capability-scoped-functions]]
- GIVEN a WASM module requests Aspen host functions
- WHEN the module is instantiated
- THEN the host SHALL expose only declared capability-scoped functions and SHALL deny undeclared KV, blob, route, network, clock, or secret access

#### Scenario: Deterministic extension mode is bounded [r[runtime-host-loading.wasm-service-host.deterministic-extension]]
- GIVEN a WASM module is used as a policy hook or service extension
- WHEN it executes inside a deterministic extension mode
- THEN the host SHALL bound fuel, memory, time, input size, output size, and ambient effects

#### Scenario: WASM service fragment declares route ownership [r[runtime-host-loading.wasm-service-host.service-fragment-routes]]
- GIVEN a WASM artifact is admitted as a runtime service fragment rather than a plugin
- WHEN it declares route ownership
- THEN the WASM host SHALL validate WASM-specific ABI and capability prerequisites
- AND `runtime-service-core` SHALL remain authoritative for route ownership, route-conflict resolution, and route-registration receipts before activation

#### Scenario: WASM failure emits redacted receipt [r[runtime-host-loading.wasm-service-host.failure-receipt]]
- GIVEN WASM validation, instantiation, execution, or host-call authorization fails
- WHEN the host records the failure
- THEN it SHALL emit a receipt with module identity, ABI, failure class, bounded diagnostics, and redacted capability summary
