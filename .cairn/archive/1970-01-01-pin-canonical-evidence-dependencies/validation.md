# Validation Evidence

## Bound implementation

- Dependency-policy implementation: `a90da2ccd7edfe4177b6235a820d94b904ca64a7`
- Pure-CI source pinning: `b470187ab532b7fdf2bc2a9a0a85c1eeb5343ac2`
- Canonical Valence revision: `5f1c2ba5072c6f9622fa59b1af20502985f569fd`
- Archived Valence task digest: `3abda77b5931c5ef6dcdde504f71dfce06f95d1d6c43f087cd35f8816147f7e2`
- Archived Octet task digest: `b3ac48ea0a241194d86293473facd24b3f380beb264a05f0334c002bb0ca2f34`
- Archived Octet cutover manifest digest: `811bf0ce699b31f981ee4dfcdae40f5b3a2152e47e607f70dfaa5b7ff0483439`

## Positive and negative validation

The following rails passed on the bound implementation and profile:

- `nix develop -c cargo test --workspace`
- `nix develop -c cargo clippy --workspace --all-targets -- -D warnings`
- Focused `molten-core` and `molten-release-policy` tests and strict Clippy
- Positive exact-pin Nickel fixture export
- Negative floating-revision and unsupported-transport Nickel fixture rejection
- Repository release-policy execution with the archived Valence and Octet evidence sources
- License-boundary validation
- Contract-export drift validation
- Canonical Cairn repository validation and proposal, design, and tasks gates
- `nix flake check -L --option secret-key-files ''`, which reported `all checks passed`

The final repository policy report passed with five dependency rows, two archive receipts, and BLAKE3 report digest `60a328729e4721a03905decfcd29468e09adbc47b6649f0961dbd693bfa733bc`.

The empty `secret-key-files` override is an operator-environment accommodation for this machine's absent configured Nix signing key. It does not alter the evaluated flake inputs, derivations, or policy decisions.

## Spec synchronization

Native Cairn sync applied the project delta with plan hash `9a6b863a470287fef75e8cda8138edaea07e0bed4d5e2c0595c6d2c3b1860b9b` and receipt hash `c2643e8fe789174fc3ee135a379cc184154b42ccb0305be8ee0b9d26011ecd75`.
