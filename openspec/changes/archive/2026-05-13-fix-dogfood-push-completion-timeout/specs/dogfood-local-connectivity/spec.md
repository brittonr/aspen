## MODIFIED Requirements

### Requirement: Git push through federation

A git push to alice's forge MUST succeed, bob MUST be able to sync the objects via federation protocol, and local dogfood runs MUST expose deterministic evidence for the first failing push boundary when the push does not complete. Local dogfood acceptance pushes MUST avoid transferring unrelated historical commits when proving current-source Forge ingestion and CI trigger acceptance.

#### Scenario: Bob syncs pushed objects

- **WHEN** a git push succeeds against alice's forge
- **THEN** bob MUST sync the pushed objects through the federation protocol

#### Scenario: Local dogfood push failure is bounded and classified

- GIVEN dogfood runs a local same-host cluster with relay servers and mDNS disabled
- WHEN the dogfood `push` stage fails or times out before build, deploy, or verify
- THEN the saved dogfood receipt SHALL identify the first failed push sub-boundary, such as local git invocation, forge receive-pack connection, object ingestion, hook dispatch, CI trigger acceptance, federation/watch publication, or push completion
- AND the failure SHALL include elapsed duration and a redacted operator-visible category without printing credential material

#### Scenario: Local dogfood push uses bounded current-source snapshot

- GIVEN dogfood-local is pushing the current Aspen source into an empty local Forge repo for acceptance
- WHEN the push workspace is prepared
- THEN it MUST contain the committed source tree as a bounded single-commit Git repository rather than the full historical object graph
- AND the push MUST still use the real `git-remote-aspen` Forge path and registered CI watch
