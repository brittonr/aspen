# Verification evidence

## Immutable inputs

Molten pins these private Radicle sources:

- executable-extent `rad://z37R1bP1kHcELs89RNbQRaqbCVKxB` at `025d9636f0161777710dac37b3c210ca0ad9483f`
- Mantle producer `rad://z3DJe8tEdQuXpzTkfqCYQq6ZUqqkb` at `2c636b1b25353a1b0befa5af48dc68615cd686dd`

Cargo manifests, Cargo.lock, Nix inputs, flake.lock, source checks, documentation, and the consumer receipt bind these revisions. No product dependency uses a sibling path.

## Baseline

Before implementation, focused `molten-core` world-commit Clippy and root world-commit integration tests passed. The baseline command completed in Pueue task 586.

## Focused Rust verification

The following final commands passed:

```text
cargo fmt --all -- --check
cargo check --workspace
cargo test -p molten-core --features executable-extents executable_extent
cargo test -p molten --features executable-extents --test executable_extent
cargo clippy -p molten-core --all-targets --features executable-extents -- -D warnings
cargo clippy -p molten --features executable-extents --test executable_extent -- -D warnings
```

The core tests include positive admission and negative identity, length, layout, target, page, closure, cohort, authority, and fallback cases. The shell tests include exact mapping, explicit unmap, capability-root access, producer drift, member drift, target and page mismatch, missing closure, unavailable authority, denied authority, path traversal, and unknown fields.

## Conformance and exact fixture

The Molten BLAKE3 1.8.5 cohort preserved these executable-extent identities:

- layout corpus: `765a7b724c3ba1215f66aee8657a728038b6608ef7e6060366e0490a628acaae`
- transition corpus: `bbf6fea58b43ccb6d8e47d4cea113b1c0a94ff8058ce14ef104d8fc857c48362`

The consumer admitted and remeasured the exact Mantle fixture:

- bundle: `2f44f5eeb1d93cafc65dfdac36fb0d2020fed4466d074d331602422a8d411d81`
- producer receipt: `82b1ccefdbe9080649c154e02410f53006cdf7ce541a3b93e653c1846aea526a`
- built artifact and extent: `4598e001cd6e4c4fe4aa57bb055c11f1cbe10b3e0def42de0da8ec4036500f6c`
- layout: `b1f86e8102e359b22e9f0cb4f7efaa65ad84f6953fc68259b725bf1051400dd9`
- detached Molten consumer receipt: `e02c5e04505f3bd05b351593d471ef64a86596b4c304a71f97d49a770dd4482f`

The mapping reached `executable-read-only` and then `unmapped`. Missing execution authority produced an inert receipt with no mapping.

## Typed contracts and Nix

The positive Nickel consumer receipt passed typecheck and export. Hostile writable-executable, sandbox-overclaim, and unknown-field fixtures failed.

These local-builder Nix gates passed:

```text
nix build path:$PWD#checks.x86_64-linux.executable-extent-consumer -L --builders ''
nix build path:$PWD#checks.x86_64-linux.executable-extent-octet-deny-all -L --builders ''
```

The source gate compares the checked Mantle fixtures with the immutable producer source. It also checks Cargo, lock, Nix, schema, hostile fixture, and no-sibling-path facts.

The dedicated full-catalog Octet gate reported:

```text
Status: clean
Findings: 0
Warnings: 0
Errors: 0
Config hash: b3:65601bc389543f706075c4421ac3422d069336c3105ec75646f87e0ef7d97247
Profile hash: b3:7a2ed279b122fed9fbdbe38829089b1fb44d67a834a21662a460a0151d2ec0bb
```

The focused workspace uses real product source. Nix replaces only transport locations with the exact immutable source inputs before strict checking.

## Scope

The default product build keeps the optional `executable-extents` feature disabled. This prevents a pilot profile from entering release roots without a release decision.

The focused gate does not clear inherited findings in unrelated Molten code. It does not prove compiler correctness, executable semantics, sandboxing, host integrity, external authority freshness, storage authority, or release eligibility.

## Lifecycle status

Strict Cairn validation passed with all 15 tasks complete. The package remains active because its accepted `world-commit` base specification still belongs to the unarchived `introduce-world-commit-core` dependency. Sync and archive must occur after that base lifecycle closes.
