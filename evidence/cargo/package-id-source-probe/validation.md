# Exact-source Cargo package-ID counterexample

## Result

The probe demonstrates a formatter panic and a separate parser rejection without a Radicle request.
Cargo schema source `4d1f984518c77fad6eeef4f40153b002a659e662` panics during pathless URL formatting.
Its parser also rejects an explicit name/version on that URL.
That rejection is a compatibility barrier, not a separately established violation of the Cargo specification.
An ordinary SSH round-trip and an invalid-version rejection control pass.
The formatter leaves the input unchanged.

The successful diagnostic disables only `RUSTC_WRAPPER` for that command inside the unchanged repository Nix environment.
It is not acceptance evidence for the normal cached build path.
No Cargo binary, toolchain selection, product manifest, product lockfile, producer pin, or accepted specification changed.

## Source and dependency boundary

- Molten environment source: `5c71c5b443423f4c601c78a5b56a89c19b658b78`.
- Cargo source: `ssh://git@github.com/rust-lang/cargo.git` at `4d1f984518c77fad6eeef4f40153b002a659e662`.
- Cargo source tree: `db1a4b289b51143a47a85bea4ddd43212fd7da32`.
- Schema crate: `cargo-util-schemas` version `0.13.2`.
- Compiler: `rustc 1.98.0-nightly (31a9463c6 2026-05-25)`.
- Cargo executable: `cargo 1.98.0-nightly (4d1f98451 2026-05-15)`.

The fetched Cargo checkout has no tracked or staged changes.
The before/after identity logs match byte for byte.
Those logs bind the upstream manifests, upstream lockfile, and package-ID implementation with BLAKE3.
The generated diagnostic lockfiles record the separately resolved dependency sets.
The exact schema source does not establish the complete dependency closure of the installed Cargo executable.

The scripts use a recorded local checkout path for the direct-path diagnostic.
That path is not a product dependency or a published component contract.
The source pin and hashes remain necessary for future reproduction.
No upstream source was copied into Molten product code.

## Executed checks

| Check | Result | Meaning |
|---|---|---|
| Immutable Git dependency probe | Exit 124 at eight minutes | No probe result. The log does not establish the full cause of the delay. |
| Exact checkout identity | Exit 0 | The requested source revision and selected file hashes match. |
| Direct-path probe with normal wrapper | Killed by the coordinator | Task-owned `kache rustc` children waited on locks. No probe result. |
| Direct-path probe without the wrapper | Exit 0 | Every positive and negative control completes. |
| Initial formatting check | Exit 1 | Repository formatting requires separate imports. |
| Formatting correction and repeated probe | Exit 0 | Formatting passes, and the same controls complete on the formatted script. |
| Final source identity comparison | Exit 0 | No upstream source or selected manifest changed. |

The original executed scripts remain in a separate archive from the formatted scripts.
The first successful compilation reports 20.76 seconds. The formatted repeat reports 0.48 seconds.
These are observations, not a benchmark or a causal comparison with earlier attempts.

The exact expected panic is:

```text
package_id_spec.rs:248:40
called `Option::unwrap()` on a `None` value
```

The parser returns:

```text
pkgid urls must have at least one path component: rad://z2QJLUqyAZnnHPiZQ1BFjLsX9ush3
```

The successful transcripts end with `cargo_or_nextest_repair_claim=false`.
The caught panic is expected counterexample output, not an unhandled probe failure.

## Environment observations

The direct-path probe had two `kache rustc` children in `locks_lock_inode_wait` during compilation requests.
A lock query showed a wait on `/var/cache/kache-nix/user-brittonr/store/gc.lock`.
The lock holder's ownership was not established. The coordinator did not control that process or alter the shared cache.

A campaign-note write also failed with `EDQUOT: unknown error, write`.
The Git dataset then had zero available space. A later query showed about 4.5 GB available without task cleanup or a quota change.
The campaign occupied about 0.22 GiB on that dataset.
Its build directory was on a separate dataset, so removal of build files did not offer a remedy for the Git quota.
The prior note remained intact. No source, evidence, worktree, cache, or user data was deleted.

The coordinator stopped only the identified direct-path probe.
The captured Pueue tool output reports a killed task.
A later JSON export returned `{}` and is not a structured terminal receipt.
There is no direct exit file for the killed command. No numeric exit code is inferred.

## Ownership and non-claims

Molten build maintainers own this diagnostic evidence and the future regression fixture.
The current consumer is Molten's blocked Cargo metadata and nextest path.
The source pin, diagnostic lockfiles, explicit controls, and lossless transcripts support repetition.
`experiment.tar.gz`, `inputs.b3`, and `payloads.b3` retain those inputs and observations.

The coordinator performed the local source passes. No independent review completed in this round.
A formatter-only repair is insufficient because explicit package identities must also parse correctly.
The installed Cargo metadata command, nextest, strict Octet, complete Nix, lifecycle acceptance, and mainline integration remain open.
This evidence grants no acceptance, migration, transport, or publication authority.
