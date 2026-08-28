# System Extension Runtime Specification Delta

## Purpose

Require exact provider output bytes when a native callback must interpret an effect completion.

## ADDED Requirements

### Requirement: Native effect completions materialize provider output

r[molten.system_extension.native_host.effect_completion_value] A native host profile that requires materialized values MUST deliver the exact bounded provider output value with its canonical effect completion.

#### Scenario: Provider output bytes are delivered

r[molten.system_extension.native_host.effect_completion_value.accepted]
- GIVEN the admitted provider returns an output schema, BLAKE3 reference, and matching bounded bytes
- WHEN the native service constructs and delivers the effect completion
- THEN the version-two completion MUST bind the exact output value
- AND the callback MUST receive those bytes through the normal materialized payload boundary
- AND Aspen MUST leave terminal interpretation to the extension

#### Scenario: Provider returns only a reference

r[molten.system_extension.native_host.effect_completion_value.rejected]
- GIVEN the active native profile requires materialized values
- AND the provider returns no bytes, changed bytes, a different reference, or an oversized value
- WHEN effect completion admission runs
- THEN the native service MUST reject callback delivery
- AND the completed provider effect MUST NOT be retried automatically
- AND Aspen MUST NOT infer terminal meaning from the reference

#### Scenario: Generic reference-only profile remains supported

r[molten.system_extension.native_host.effect_completion_value.compatibility]
- GIVEN a non-materializing generic system-extension profile admits reference-only outputs
- WHEN its effect completion is constructed
- THEN the canonical record MAY contain no materialized output
- AND the profile MUST NOT present that record as a materialized native completion
