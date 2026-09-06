# Prerequisite checkpoint — 2026-09-06

This checkpoint restores source availability for the current Molten checkout. It does not prove normal-node content serving.

## Exact inputs recovered

The desktop owner repositories contain both requested commits. Read-only SSH discovery confirmed their Radicle remotes. Fetch copied only Git objects into a new local cache.

| Input | Git commit | Locked source NAR |
| --- | --- | --- |
| executable-extent | `025d9636f0161777710dac37b3c210ca0ad9483f` | `sha256-gN8jGxgxwckXu/YyG/p2YGkQnPrsJZPfkCy5t3r3yo4=` |
| vm-cohort | `31f1696ba9391bfda8577a58af84f72361d5573e` | `sha256-XLBmSBNJg9D4sZJ9krzkPz46ohww/pqVNxSgIvUNfn4=` |

Both exported trees matched the unchanged `flake.lock` NAR hashes. Invocation-local Git URL rewrites supplied these exact objects to Cargo. Neither canonical source URLs nor revision pins changed. No global Git configuration changed. The new cache initially lacked a valid HEAD. Setting its HEAD to the exact pinned ref fixed Cargo fetch.

`cargo fetch --locked` then passed. Subsequent offline compilation did not require the cache-transport overrides. `Cargo.toml`, `Cargo.lock`, and `flake.lock` remain unchanged. Private source trees and Git caches remain outside this repository.

## Current-source baseline

Rust 1.95.0 rejects current dependencies that require Rust 1.96.0. An existing installed Rust 1.97.1 compiled the unchanged current library and ran both content example tests successfully. No compiler was installed, rebuilt, or replaced.

```text
test tests::archive_identity_and_output_are_not_replaceable ... ok
test tests::handoff_admission_checks_membership_bounds_and_independent_expectations_without_network ... ok
test result: ok. 2 passed; 0 failed
```

The contained baseline took 6 minutes 38.030 seconds and peaked at 7.9 GiB. The selected example Clippy check passed with `-D warnings` in 3 minutes 26.243 seconds. The earlier Rust 1.95 lint finding did not reproduce with this compatible compiler. No lint suppression or production Rust edit was needed.

The current content core also passed all 8 selected tests. The normal `molten` CLI compiled successfully in the isolated target directory. That build is preparation, not evidence that the normal node serves content.

This compiler is not the repository's exact `nightly-2026-05-26` toolchain. These checks do not replace the pinned Octet gate or establish full workspace acceptance.

## Cairn remains blocked

The project pins Cairn `3b4c280b893f2709aebea21fc51a4f9eeba3fe3b`. That exact CLI was built separately with the existing Rust toolchain. It accepts the old policy shape, but looks for changes under `cairn/`. Active changes now live under `.cairn/`. The existing `cairn/archive/` is retained history, not an empty alias location.

The installed current CLI recognizes `.cairn/`, but requires newer traceability and workflow records. A bounded experiment added the owner's required `declared` assurance and `exact_marker_bytes` anchors. It then exposed the larger workflow-schema mismatch at `schemas`, beginning with missing `output_path`.

All experimental policy edits were reverted. Neither lifecycle tree was replaced, moved, hidden, or aliased. No policy default, requirement, or evidence hash was weakened. A reviewed schema/layout migration remains necessary before producer gates can pass.

## Scope

No normal node service or new VM run started in this checkpoint. No host blob service, physical deployment, Stage0, Darkhttpd rebuild, Mantle rebuild, or package-toolchain rebuild occurred. The existing fixture proof and its immutable identities remain unchanged.

The next implementation must use the normal `node serve` lifecycle, not relabel the example driver as a production daemon. Its two-VM behavior and native replay remain open tasks.
