# Manifest and Tier Admission

System-extension admission is a conjunction of independently meaningful boundaries, not a promotion granted by an executable name. This article assumes familiarity with the three executable tiers in the [system-extension runtime](../../system-extension-runtime.md) and explains how a manifest becomes an admitted service description. It complements, rather than replaces, those governing requirements. Return to the [Technical companion](../README.md) for related topics.

## Admission is not a single credential

A sandboxed plugin, a system extension, and an application workload have different authority roles. Plugin permission evidence cannot turn a plugin into a system service; application permission to use a service cannot become permission to own its protocol or adapters. The [plugin lifecycle FSM](../../plugin-lifecycle-fsm.md) makes a related distinction: possession of receipts supplies inputs to an admission relation, rather than ambient authority. Its plugin-specific transitions are not the system-extension transition table.

In the core, `validate_extension_tier` accepts an `ExtensionTierRequest` and produces an `ExtensionTierAdmission`. It checks collection bounds, duplicate declarations, authority/tier compatibility, and required evidence categories. System-extension evidence includes manifest, policy, provenance, explicit port bindings, resource grants, and lifecycle admission. The admitted authority list is the sorted requested list, not an implicit grant of every system-level facility. These are deterministic checks over supplied facts; the function does not contact a policy service or inspect executable files. See the [tier implementation](../../../crates/molten-core/src/fabric/tier.rs).

Manifest admission is a second boundary. `SystemExtensionAdmissionContext` supplies a port registry, a tier admission, and the admitted execution profiles. `admit_system_extension_manifest` validates the input against that context and returns either an admitted manifest or accumulated issues. Separating these inputs matters: a well-formed manifest is not evidence that its requested execution profile or authority has been admitted in this deployment.

## What becomes bound

The [manifest implementation](../../../crates/molten-core/src/system_extension/manifest.rs) checks identity, callback groups, reference sets, state compatibility, resources, non-claims, and ports. Required callbacks are `initialize`, `start`, `drain`, and `shutdown`. Unknown callbacks and duplicate declarations are rejected, rather than interpreted as future functionality. Capability, policy, and provenance reference sets must be nonempty and syntactically valid. State compatibility explicitly includes the current schema itself.

The admitted representation retains both port requirements and resolved bindings. This is important for subsequent effect validation: resolving a provider does not erase the narrower operation and schema requirements of this manifest. Every authority named by a port requirement must occur in the supplied tier admission. Required ports must resolve; the input cannot omit all required ports.

Optionality has a precise meaning. If no descriptor has the optional requirement's port identifier, the binding remains absent. If that identifier is available, resolution is attempted, and incompatibility produces `OptionalPortDenied`. An incompatible version does not make the port conveniently disappear. Duplicate `(port-id, version)` keys across required and optional declarations also deny. This avoids ambiguity about which declaration authorizes an effect.

Successful admission sorts callbacks, requirements, bindings, and several reference collections. This stabilizes the admitted representation; it does not establish identity from Rust object layout. The governing runtime describes canonical Preserves/BLAKE3 manifest evidence at the shell boundary. Pure validation and canonical evidence production remain distinct responsibilities.

## Worked admission failure

Consider an illustrative archival service. Its required storage port is admitted for a specific version and operation cohort. It also declares an optional diagnostic port at version 2. The registry contains that diagnostic identifier only at version 1.

The storage binding may resolve successfully, and the diagnostic functionality may be dispensable to the application. Nevertheless, this input fails optional-port admission: the identifier is available, but the declared binding does not resolve. Omitting the diagnostic provider entirely would produce a different admission result, because absence and incompatible presence are different facts. Silently substituting version 1 would change the interface contract behind the manifest.

Now suppose an operator supplies a valid plugin manifest reference alongside this service executable. That cannot repair system-tier evidence. Likewise, adding a capability reference string cannot enlarge `admitted_authorities`. The useful diagnosis is the failed boundary—tier evidence, binding compatibility, or reference shape—not a generic assertion that the artifact is trusted.

## Review and suggested verification

Review admission as a matrix: executable tier, authority set, profile, callback declaration, exact port cohort, resource envelope, and state compatibility. For each dimension, ask what supplied fact is checked and what external evidence would establish that fact. Review the existing core cases for plugin metadata rejection, unadmitted profiles, required callback omissions, and incompatible required ports in the [system-extension tests](../../../crates/molten-core/src/system_extension/tests.rs).

Suggested verification is to vary one admission input at a time and observe the issue category while keeping the others fixed. For optional ports, compare absence with incompatible presence. These are review instructions, not a report of test execution for this article.

## Limits and non-claims

Admission neither activates code nor proves its semantics. Reference validation does not establish the truth of provenance, and native execution is not sandboxing. A native executable additionally passes the admission and materialization boundaries described by the [native host](../../native-system-extension-host.md). No manifest or successful admission result proves durable state, correct external effects, distributed availability, or production readiness.

## Sources

- [System-extension runtime](../../system-extension-runtime.md)
- [Native system-extension host](../../native-system-extension-host.md)
- [Plugin lifecycle FSM](../../plugin-lifecycle-fsm.md)
- [Tier admission implementation](../../../crates/molten-core/src/fabric/tier.rs)
- [Manifest admission implementation](../../../crates/molten-core/src/system_extension/manifest.rs)
- [Core admission and lifecycle tests](../../../crates/molten-core/src/system_extension/tests.rs)
