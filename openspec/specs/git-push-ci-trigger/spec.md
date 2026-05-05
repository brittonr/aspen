# git-push-ci-trigger Specification

## Purpose

Defines the Git Push CI Trigger capability requirements preserved by Aspen's archived OpenSpec records.

## Requirements

### Requirement: Bridge push announces updated refs

`handle_git_bridge_push` MUST call `announce_ref_update` for each successfully updated ref.

#### Scenario: Bridge push announces updated refs works

- **WHEN** a git bridge push updates one or more refs
- **THEN** each successfully updated ref MUST be announced

### Requirement: Announcement includes new hash

The announcement MUST include the blake3 commit hash as `new_hash`.

#### Scenario: Announcement includes new hash works

- **WHEN** a ref update announcement is emitted
- **THEN** the announcement MUST carry the new blake3 hash

### Requirement: Announcement includes old hash

The announcement MUST include the previous blake3 hash as `old_hash` when the ref existed before, or `None` for new refs.

#### Scenario: Announcement includes old hash works

- **WHEN** a ref update announcement is emitted for an existing or new ref
- **THEN** the old hash MUST be populated for existing refs and omitted for new refs

### Requirement: Announcement failures do not fail pushes

Announcement failures MUST NOT cause the git push to fail.

#### Scenario: Announcement failures do not fail pushes works

- **WHEN** CI ref announcement fails after a successful ref update
- **THEN** the push MUST still complete successfully

### Requirement: Federation dogfood enables CI

The federation dogfood script MUST set `ASPEN_CI_FEDERATION_CI_ENABLED=true` for bob's cluster.

#### Scenario: Federation dogfood enables CI works

- **WHEN** the federation dogfood script starts bob's cluster
- **THEN** the CI federation environment flag MUST be enabled
