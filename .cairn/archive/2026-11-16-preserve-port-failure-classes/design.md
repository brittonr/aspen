## Context

`Failure` had four variants and almost every error used `InvalidHarness(String)`. `FabricPortError` already distinguished seven infrastructure classes, but its conversion into `Failure` stringified them, so a class survived only as text.

## Decisions

### Decision: Carry the class in a dedicated `Failure` variant

**Choice:** Add `FailureClass` and `Failure::Classified { class, message }`. `From<FabricPortError>` maps each port variant to its class and keeps the message; `From<Failure>` restores the exact port variant from a classified failure and treats other failures as capability failures, as before. `Failure::class()` exposes the class, and `FailureClass::code()` gives a stable identifier for reports.

**Rationale:** The class is preserved through `?` without embedding `FabricPortError` in the host crate, which does not depend on the fabric module. Shells branch on `class()` rather than parsing text.

### Decision: Rename the displayed prefix, not the message

**Choice:** A classified failure displays as `<class>: <message>`, the same form `FabricPortError` already used.

**Rationale:** Operators no longer see port failures labelled "invalid harness artifact". Text consumers that parsed the old prefix are updated in the same change.

## Risks / Trade-offs

- Callers that construct `InvalidHarness` directly still produce unclassified failures; only port conversions and the native lifecycle uncertainty path are classified here.
- Display text for port failures changes; anything matching the old "invalid harness artifact:" prefix for those failures must follow the class instead.
