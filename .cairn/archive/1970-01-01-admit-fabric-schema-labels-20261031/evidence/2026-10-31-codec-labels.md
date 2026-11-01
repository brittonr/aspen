# Codec label admission (2026-10-31)

Worktree `/home/brittonr/.cargo-target/aspen-codec-fix-20261031`, base `b5197b2a9`. Commands ran
inside `nix develop path:. --builders ''` with task-owned cache and temp directories.

## Defects

1. `src/node/servicehost/codec/wire.rs::field` treated every `*_ref`/`*_refs` field as a content
   reference. Fabric port descriptors and the collaboration commit effect name their schemas by
   contract label, so a bundle carrying a real admitted manifest encoded but could not decode:
   `content ref must start with blake3:, got molten.fabric.transport.command.v1`.
## Changes

- `field` now excludes `schema_ref`/`schema_refs` names from the content-reference check, in
  addition to the existing `max_*` exclusion.

## Checks

| Command | Result |
|---|---|
| `cargo test -p molten --lib node_service_host::codec` | 7 passed, including `fabric_schema_labels_are_not_content_references` |
| `cargo test -p molten --lib node_service` | 18 passed |
| `cargo fmt --check` | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `cargo octet check` (primary) | `Status: clean`, `Findings: 0` |

## Non-claims

The fix changes no admission, authority, trust or release eligibility. The producer build that
exercised it is recorded with the change that needed it (`route-installed-native-effects`).

## Addendum: hermetic cohort vendoring shipped with this commit

The same commit also binds the retained Octet cohort's Cargo git vendoring to
`crate-hashes.json` (`nix/startup-octet-cohort.nix` `outputHashes`). The collaboration native
effect rail needed it because `git.onix.computer` returned HTTP 530: crane's
`builtins.fetchGit` fallback made the producer unevaluable even though every source identity was
recorded. The fix was verified by a real producer build in this worktree
(`octet gate pass receipt=blake3:ad0bfb3d…418a3b30`) while that host was unreachable. It is a
build-hermeticity fix only and claims no admission, authority or release behavior.
