# Cargo package-ID compatibility

## Purpose and owner

Molten's toolchain maintainers own this consumer patch.
Cargo remains the owner of package-ID syntax and implementation.
The source is `rust-lang/cargo` at revision `4d1f984518c77fad6eeef4f40153b002a659e662`.
Nix pins that source and applies `pathless-package-id.patch`.

The patch prevents a formatter panic for source URLs without a final path component.
It preserves explicit package names instead of adding a false URL path.
If the parser must infer the name from a path, it requires that path.
Existing fragment disambiguation and validation rules remain unchanged.

This patch does not rewrite metadata, replace dependencies, authorize a transport, or change Molten runtime behavior.
The recipe composes the repaired Cargo with the original compiler and target libraries.
The binary description includes `molten-pathless-package-id`.
Its output also retains the patch, upstream revision, and upstream license notices.

## Registry downloads

The pinned Nix import helper uses the legacy crates.io API download route.
That route returned HTTP 403 during this repair.
The recipe uses the download endpoint declared by the crates.io index: `https://static.crates.io/crates`.
Only the download URL changes. Registry identities, versions, and upstream Cargo.lock checksums remain intact.
The helper must not add a second source alias for the default registry.
Cargo's checksum format requires SHA-256. Molten repair evidence uses BLAKE3.

## Verification

Build the package:

```sh
nix build .#cargo-pathless --no-write-lock-file
```

Run the tool-selection check:

```sh
nix build .#checks.x86_64-linux.cargo-pathless-toolchain --no-write-lock-file
```

Inspect the tools from the repository environment:

```sh
nix develop --no-write-lock-file -c cargo -Vv
nix develop --no-write-lock-file -c rustc -Vv
```

Run the affected metadata command:

```sh
nix develop --no-write-lock-file -c cargo metadata --format-version=1 --all-features --filter-platform x86_64-unknown-linux-gnu --locked
```

Run nextest:

```sh
nix develop --no-write-lock-file -c cargo nextest run --locked
```

The package check includes the upstream schema suite and the added positive and negative cases.
The tool-selection check rejects the unpatched Cargo identity.
It verifies the compiler, rustfmt, Clippy executables, and the complete Rust library tree against the original toolchain.
Schema results do not replace actual metadata, nextest, or the repository's other required gates.

## Maintenance

Keep the source revision, patch, compiler cohort, and validation results together during an update.
After a selected upstream Cargo passes the same regressions and actual consumer checks, remove this patch.
A compiler migration requires a separate review of the compiler and target-library changes.
No sibling checkout or global Cargo replacement forms part of this package.
