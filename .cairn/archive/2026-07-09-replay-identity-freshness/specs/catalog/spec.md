## ADDED Requirements

### Requirement: Release readback denies stale replay identity
r[molten.release.replay_freshness.readback] Release and dogfood readback SHOULD deny replay verify or replay index evidence whose run identity does not match the release or dogfood subject identity it claims to cover.

#### Scenario: Changed artifact ref denies release readback
- GIVEN release evidence with a replay index recorded for a different artifact ref
- WHEN release readback validates replay freshness
- THEN readback emits a deny receipt
- AND diagnostics identify the stale artifact component.

#### Scenario: Missing identity denies readback
- GIVEN release evidence with a replay verify receipt that lacks required run identity binding
- WHEN release readback validates replay freshness
- THEN readback denies before accepting the replay evidence as release review material
- AND diagnostics identify the missing identity field.

### Requirement: Replay identity is catalog-searchable
r[molten.catalog.replay_freshness.identity_search] The catalog SHOULD classify replay verification receipts, replay indexes, and release replay bindings by run identity ref, artifact ref, handler profile ref, policy refs, replay profile, freshness decision, and stale-component diagnostics when present.

#### Scenario: Search finds replay evidence by identity
- GIVEN imported replay evidence with a run identity ref
- WHEN catalog search filters by that run identity ref
- THEN matching replay evidence is returned as read-only discovery evidence.

### Requirement: Replay freshness readback remains evidence-only
r[molten.catalog.replay_freshness.evidence_only] Replay freshness receipts and catalog search results MUST NOT grant authority, policy admission, provenance trust, source-gate acceptance, release promotion, transport trust, resource rights, retention authority, or execution trust.

#### Scenario: Fresh replay does not replace source gate
- GIVEN replay freshness validation passes for a release subject
- WHEN release promotion evaluates source-gate requirements
- THEN the fresh replay evidence remains insufficient by itself
- AND source-gate evidence is still required separately.
