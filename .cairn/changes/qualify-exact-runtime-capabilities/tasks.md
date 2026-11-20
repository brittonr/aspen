## Implementation

- [x] [serial] Define reviewed capability mapping and exact report subject contract. r[molten.testing.capability_qualification.exact_evidence]
- [x] [serial] Replace generic wrapper admission with existing native artifact validation and complete conflict-preserving obligations. r[molten.testing.capability_qualification.exact_evidence]
- [x] [serial] Add current-process signed-source native scenario producer and independently observed read-only subject readback. r[molten.testing.capability_qualification.exact_evidence]
- [x] [serial] Add two actual producer scenario definitions in the existing evidence matrix. r[molten.testing.capability_qualification.exact_evidence]
- [x] [serial] Add executable native/consumer regression controls, retained signed-source fixture and command documentation. r[molten.testing.capability_qualification.exact_evidence] r[molten.testing.capability_qualification.diagnostic_output]

## Validation and integration

- [x] [serial] Parent integrates native facade/evidence CLI mounting and routing. r[molten.testing.capability_qualification.exact_evidence] r[molten.testing.capability_qualification.diagnostic_output]
- [x] [serial] Parent runs focused native tests, standalone controls and required source/lifecycle checks after integration. r[molten.testing.capability_qualification.exact_evidence] r[molten.testing.capability_qualification.diagnostic_output]
- [x] [serial] Execute the retained consumer fixture only with an independently admitted signed output containing this producer; retain exact artifacts and actual command outcomes. r[molten.testing.capability_qualification.exact_evidence]
- [x] [serial] Do not mark unexecuted product families, external checker cohorts or release/live qualification complete. r[molten.testing.capability_qualification.exact_evidence] r[molten.testing.capability_qualification.diagnostic_output]

The [earlier integrated execution](../../../evidence/integrated-cutover-2026-11-16/earlier-checks.json) records the passing complete workspace test selection and actual standalone capability-report self-test, while preserving its failed Clippy attempt. The [final ordered execution](../../../evidence/integrated-cutover-2026-11-16/checks.json) separately records the repaired source gate, workspace library tests, Clippy, formatting and strict provider-policy Cairn validation.

The signed-source consumer fixture was executed on 2026-11-20 against the realized `molten-node-source-binding-v1` output selected by a trust record provisioned outside the consumer. The [retained run](evidence/2026-11-20-experimental-signed-consumer-run/run.json) binds the experimental authority, output, selection, two native scenarios and byte-identical canonical report. The signing key was explicitly experimental, not an independently authorized production operator key; only `harness-runtime-capability-binding` is `bounded-observed`, and six product rows remain `unsupported`. This does not grant production admission or archive authority.
