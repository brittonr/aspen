## ADDED Requirements

### Requirement: Closed storage classification domains
r[molten.node_host.closed_domains] Molten MUST treat LocalStoreKind, NodeStateNamespaceKind, and NodeStateFileObservation as deliberately closed domains whose future variants require explicit consumer decisions. This change MUST NOT mark other enums or introduce fallback arms.

#### Scenario: New variants require consumer changes
- GIVEN any one of the three actual enum declarations gains a variant in an isolated source copy
- WHEN existing consumers compile without corresponding match changes
- THEN compilation MUST fail with E0004 at the relevant mapping or observation consumer sites
- AND active analysis markers MUST NOT remove this compiler protection.

### Requirement: Storage identity and observation compatibility
r[molten.node_host.closed_compatibility] Closed-domain declarations MUST preserve existing names, variants, payloads, re-exports, directory mappings, namespace registry order, error behavior, and capability authority.

#### Scenario: Fixed namespace aliases remain distinct
- GIVEN the current 14-kind namespace registry and eight local-store mappings
- WHEN contract tests enumerate their reviewed expected pairs
- THEN all current pairs and registry order MUST match
- AND Identity/Secrets and ControlIngress/Ingress MUST retain their directory aliases without collapsing distinct kind identities or authorizing another namespace view.

#### Scenario: File observation retains explicit denial
- GIVEN a missing, non-regular, or acquired regular-file observation
- WHEN bounded-read or mode policy evaluates it
- THEN absence, denial, and regular-handle use MUST retain their current distinct behavior, messages, bounds, and effect order
- AND no future observation may inherit a wildcard fallback.

### Requirement: Explicit analysis-only marker integration
r[molten.node_host.closed_marker_integration] Molten MUST register and apply the exact reviewed Octet sealing attribute only during active Octet analysis, without imposing an unconditional new compiler-feature requirement on ordinary compilation or weakening diagnostic severity.

#### Scenario: Tool and ordinary compilation remain distinct
- GIVEN the reviewed conditional registration and expected-cfg declaration
- WHEN the retained Octet driver checks the crate
- THEN exactly the three reviewed sealing declarations MUST be recognized
- AND when ordinary compilation runs without active Octet cfg, it MUST NOT require the registration feature solely because of these declarations.

#### Scenario: Spoofed declarations confer no status
- GIVEN marker-looking docs, a wrong namespace, or an inactive conditional marker
- WHEN marker admission runs
- THEN it MUST NOT establish a closed-domain declaration from that input
- AND genuine complete attribute paths MUST remain accepted under the reviewed diagnostic cohort.

### Requirement: Bounded verification and unchanged startup authority
r[molten.node_host.closed_evidence] Acceptance MUST retain actual mapping, capability, compiler-mutation, package-test, and frozen-source diagnostic evidence with tool/source identities and unchanged canonical command scope. Acceptance MUST NOT imply runtime or startup authority.

#### Scenario: Expected diagnostic reduction is scoped
- GIVEN all domain and compatibility controls pass
- WHEN the unchanged canonical command checks a fresh frozen implementation source
- THEN removal of the four reviewed enum findings MUST be attributed to the explicit domain declarations
- AND other diagnostic differences MUST be explained rather than suppressed
- AND incomplete workspace coverage and startup denial MUST remain explicit.

#### Scenario: No bootstrap or deployment effects
- GIVEN this declaration-only change is being implemented or verified
- WHEN required tooling or evidence is unavailable
- THEN work MUST stop or report the missing input without Stage0, compiler/toolchain acquisition, guard bypass, host serving, or VM/physical deployment under this change.
