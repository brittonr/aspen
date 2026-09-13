# Repair Cargo pathless package identities

## Why

Molten's pinned Cargo panics while serializing metadata for admitted Radicle dependencies.
The exact-source counterexample also shows that explicit pathless identities cannot round-trip through the current parser.
This blocks nextest before tests start. Producer revisions and transports must not change to hide the failure.

## What Changes

- Apply a narrow patch to Cargo revision `4d1f984518c77fad6eeef4f40153b002a659e662`.
- Preserve explicit package names for pathless source URLs during formatting and parsing.
- Build the patched Cargo through Nix and compose it with the unchanged Rust compiler and tools.
- Verify existing URL behavior, malformed-input rejection, real Molten metadata, and nextest.

## Impact

Molten build maintainers own the consumer patch, Nix integration, and regression checks.
The immediate consumer is Molten's metadata and CI test path.
The durable capability is a pinned, reproducible toolchain repair with executable compatibility checks.
The patch can retire after an admitted upstream Cargo revision supplies equivalent behavior.

The change does not alter runtime logic, producer revisions, accepted transport policy, global tools, or unrelated worktrees.
No metadata rewriting, dependency substitution, fixture-only pass, or warning-only gate counts as completion.
