## ADDED Requirements

### Requirement: Diagnostic qualification preserves exact execution boundaries
r[molten.testing.capability_qualification.exact_evidence] A diagnostic capability report MUST name exact source, build, dependency closure, features, execution profile, adapters, platform and environment. It MUST derive test definitions from the evidence matrix, require every named requirement's applicable positive and negative family or reviewed exemption, and preserve missing evidence and canonical failures. Caller-authored success wrappers MUST NOT establish execution or exact subject binding.

#### Scenario: Generic success conflicts with a canonical failure
- GIVEN a success wrapper and a native canonical failed artifact
- WHEN a capability is reported
- THEN the wrapper does not qualify execution
- AND the native failure remains visible

#### Scenario: Native success lacks exact build attestation
- GIVEN a native validated simulation report without exact source/build/closure/features/platform binding
- WHEN a selected live or simulation build is reported
- THEN execution remains inconclusive
- AND the missing producer binding is explicit

### Requirement: Qualification output is stable and non-authoritative
r[molten.testing.capability_qualification.diagnostic_output] Reports MUST provide stable canonical machine output and compact summaries, separate implementation from execution, and retain finite scenarios, distinct environments, evidence refs, diagnostics and caveats. Reports MUST NOT grant authority or change release admission.

#### Scenario: Identical inputs reproduce the report identity
- GIVEN identical mapping, matrix, selected subject and artifact bytes
- WHEN the report is rendered twice
- THEN canonical machine bytes and report refs match
