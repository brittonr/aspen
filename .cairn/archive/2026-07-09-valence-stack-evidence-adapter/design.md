## Design

The adapter is a pure `molten-core` decision surface over in-memory stack evidence members and Valence role/schema policy rows. The shell may load Valence policy files or generated JSON later, but the core validation remains deterministic and side-effect-free.

### Decisions

1. **Valence vocabulary is imported, not redefined.** Molten maps local `StackEvidenceRole` values to Valence role/schema rows and fails closed when a role is missing, duplicated, unsupported, or stale.

2. **Artifact refs use shared identity profiles.** BLAKE3 refs remain public Molten DTO fields, but parsing and compatibility should align with the Valence shared evidence identity core once available.

3. **Molten keeps authority boundaries.** A valid stack evidence adapter row is evidence-only. It does not grant runtime authority, release promotion, transport trust, storage trust, UCAN authority, or permission to bypass subsystem gates.

4. **Migration is fixture-driven.** Existing `molten-core::stack` tests become the seed positive and negative fixture matrix. New fixtures must cover both local validation and Valence vocabulary compatibility.

### Validation shape

Positive tests cover a complete Basalt/UCAN/Trellis/Octet/Valence/Cairn/Mantle evidence-only envelope and Valence role mapping. Negative tests cover missing role, duplicate role, malformed artifact ref, unsupported schema, missing verification role, missing evidence-only non-claim, overbroad authority claim, and Valence vocabulary mismatch.
