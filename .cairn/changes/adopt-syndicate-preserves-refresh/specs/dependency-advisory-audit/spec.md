# Syndicate Preserves Refresh Delta

## ADDED Requirements

### Requirement: A transitive fix is adopted with its whole pin surface [r[molten.dependency_audit.pin_surface]]

When a fix for a transitive advisory requires moving a pinned Git source, the
change MUST move every manifest, Nix input, release-profile revision, focused
workspace, notice and document that carries the revision in one change, MUST
re-resolve the lock without drifting unrelated package versions, and MUST re-run
the pinned audit to record the before and after.

#### Scenario: Partial repin

- GIVEN only some of the manifests or the flake input carry the new revision
- WHEN the audit runs
- THEN the release guard reports duplicate identity or Nix drift and denies

#### Scenario: Complete repin

- GIVEN every carrier of the revision moved together and the lock re-resolved
- WHEN the audit runs against the same pinned database revision
- THEN every prerequisite passes and the advisory set shrinks by the fixed entry
