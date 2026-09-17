# Policy export repair and Cargo isolation

## Outcome and owner

The policy-export drift gate now passes without a policy-content change.
Molten's typed `cairn-policy/consumer.ncl` composes the published Cairn policy with the existing root-crate trace source.
The Nix package pins Cairn revision `695124d459574ba7aeba6097310d237f393c243c`.
Cairn owns the base policy. Molten owns the customization and the byte-comparison gate.

The immediate outcome is a passing policy-export gate.
The durable capability is a repeatable, typed export from declared immutable inputs.
The positive export and two negative consumer fixtures exercise that boundary.
No ambient sibling source or copied producer implementation becomes a product dependency.

The exported and accepted policy files both have BLAKE3 identity:

```text
8b3cad867b949d361987303a2528b0ed2bf0634b36c93b5ef2dbb2ee3ff52c8a
```

`generated/cairn-policy.json`, `Cargo.toml`, and `Cargo.lock` remain unchanged.
Nix generated the new `cairn-policy-src` lock node and its root reference.
No other lock node changed. No policy field was removed.
The legacy Nickel files remain inputs for their existing fixtures.

## Validation

- The original `contract-export-drift-gate` failed before this repair.
- The new `cairn-policy-export` package built successfully.
- The repaired `contract-export-drift-gate` passed, including accepted exports and rejected Nickel fixtures.
- Both direct negative runs exited 1. They reported a missing `profiles` definition and a broken `profiles` contract.
- `cmp` confirmed exact agreement between the Nix export and the accepted policy.
- Native Cairn validation passed with the explicit checkout root and selected policy.
- Proposal, design, and tasks gates passed. These are structural checks, not implementation acceptance.
- Nix formatting and Git whitespace checks passed.
- The broader Nix pass reached optimized Molten compilation without another recorded build error, then exhausted its five-minute budget with exit 124.

The broader Nix gate is not complete. The scoped export result is not release evidence.
No Rust behavior changed in this repair. Earlier Rust checks remain scoped to the earlier implementation.

## Strict-check isolation

The official deny-all hook came from the existing Octet pin:
`cf04e894e53eb0947230118a086ef6066ddba38c`.
It ran with workspace, all-target, and all-feature arguments.
It exited 2 with `Status: integration-failure` after the Cargo worker panicked.
Zero findings from that failed producer do not establish strict acceptance.

Independent of Octet, ordinary Cargo metadata commands produced these results:

| Feature selection | Exit | Observation |
|---|---|---|
| Default | 0 | Metadata succeeded |
| `doltlite-oracle` | 0 | Non-Rad optional-feature control succeeded |
| `executable-extents` | 101 | Package-ID formatting panicked |
| `world-snapshot-vm-cohort` | 101 | Package-ID formatting panicked |
| All features | 101 | Package-ID formatting panicked |

These are local diagnostic passes, not independent agent reviews.
The two failing feature groups select the existing `rad://` dependencies.
The dependency URLs and revisions remain unchanged.

The plain Cargo control used `cargo 1.98.0-nightly`, commit `4d1f984518c77fad6eeef4f40153b002a659e662`.
The panic points to `cargo-util-schemas/src/core/package_id_spec.rs:248:40`.
The official Octet invocation reported the same failure family with its March toolchain.

The exact Cargo source is available at:
https://raw.githubusercontent.com/rust-lang/cargo/4d1f984518c77fad6eeef4f40153b002a659e662/crates/cargo-util-schemas/src/core/package_id_spec.rs

Its package-ID formatter uses `url.path_segments().unwrap().next_back().unwrap()`.
Its parser separately rejects a URL without a path component.
A toolchain repair therefore needs formatting and round-trip parsing controls for named Rad packages.
It must retain non-Rad behavior and reject pathless inputs without an explicit package name.
A formatter-only change is not yet a validated repair.

## Remaining boundary

Strict all-feature acceptance requires a Cargo package-ID repair or a separately reviewed dependency-source migration.
This pass does not change those source identities, disable features, suppress lints, or bypass hooks.
The full Nix gate also remains incomplete after its budget expired.
The F12 change remains active. No sync, archive, or integration is claimed.

Run `b3sum --check digests.blake3` from this evidence directory.
The manifest binds the report and compressed logs, including failed and budget-exhausted runs.
