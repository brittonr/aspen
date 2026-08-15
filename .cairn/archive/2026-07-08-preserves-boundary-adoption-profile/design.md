## Context

Molten's README already names Preserves + BLAKE3 as the communication, storage, policy, and evidence boundary. The missing piece is a profile that distinguishes required boundary artifacts from implementation internals.

## Design

Define a Preserves boundary profile with rows for artifact family, schema label, canonical-byte requirement, BLAKE3 identity field, adapter owner, core DTO, allowed consumers, and non-claims. The pure checker validates parsed profile rows and measured artifact metadata. The CLI/test shell reads files and measures bytes.

Core modules SHOULD accept typed DTOs, not raw Preserves values, except in small adapter/facade modules. Negative checks should reject new core dependencies on raw Preserves values when the profile marks the surface as adapter-only.

## Non-claims

Passing this profile proves canonical envelope identity and boundary placement only. It does not prove transport liveness, actor authority correctness, replay completeness, or Valence Evidence IR acceptance.
