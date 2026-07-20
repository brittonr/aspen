# Validation: Molten artifact-auth non-production canary evidence

Validated on 2026-07-20 against Molten `19549c83f9cb046f0f6f4adebd4fa02f2b936deb` and artifact-auth `799459346d5416fbd7b9f55840a7371441b55afa`.

## Baseline

- Cairn validation passed before this evidence-only change.

## Evidence checks

- The self-contained public subset contains 11 regular files: the exact harness, operational receipt, capture/replay/rotation/post-rotation summaries, post-rotation log, typed manifest, bounded hash script, README, and inventory.
- `nix shell nixpkgs#nickel -c nickel typecheck .../manifest.ncl` passed.
- BLAKE3 inventory regeneration reproduced byte-for-byte; the negative symlink fixture failed with `symlink is forbidden`.
- JSON/log secret scanning found no private-key marker, serialized secret-key field, or long name-prefixed private-key value.
- The generation-1 receipt retains `standalone_authority_admitted = false`; the rotation summary contains the expected current-key-state drift denial.
- A fresh process observed generation 2 with a changed handle versus the generation-1 receipt.

## Lifecycle checks

- Cairn validate plus proposal, design, and tasks gates passed.
- Sync dry-run passed with receipt `2666829ff20f0827768e143460c0f490b5f9f63cdea9b0bd7cb45250662b60c6`.
- Sync execution passed with receipt `577c0edf522edb88b0ce97c2030557ef2da333d45b15cb974e59c251f30af1c7`.
- Accepted requirements `molten.artifact_auth_operational_receipt.canary_archive` and `molten.artifact_auth_operational_receipt.canary_authority` are present.

## Adversarial audit

The advisory audit challenged self-containment and cross-consumer identity binding. Cross-consumer revisions are now explicitly review linkage rather than a joint signature or attestation. Its suggestion to clear `legacy_authoritative` was rejected because retaining legacy authority is the required fail-safe while standalone authority remains unadmitted. Deterministic checks remain authoritative.

## Bounded result

The exact non-production capability-file replay and generation-rotation evidence is durably reviewable without private key or node-state material. This archive does not establish network revocation freshness, membership, capability, federation, transport, storage, lifecycle, signing-policy, release authority, production rollout, or standalone authority. Legacy authority and rollback remain active.
