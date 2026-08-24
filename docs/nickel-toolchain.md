# Nickel toolchain cohort

Molten uses one reviewed Nickel cohort for embedded and command-line evaluation.

The cohort is:

- Nickel CLI `1.17.0`
- upstream commit `1320a983e6c3d1e2fb53dd2464b084b4903b1426`
- `nickel-lang 2.2.0`
- `nickel-lang-core 0.18.0`
- `nickel-lang-parser 0.3.0`
- `nickel-lang-vector 0.2.0`

`flake.nix` pins the upstream CLI source and selects its package for all Nix checks and the development shell.
`Cargo.toml`, `Cargo.lock`, and the generated unit2nix plans bind the embedded evaluator cohort.

The `nickel-toolchain-cohort` Nix check rejects an old CLI, a mixed crate cohort, a stale source revision, or failed positive and negative fixtures.
The negative fixtures cover malformed refs, contract bounds, unsupported metadata, missing adapters, and missing imports.

## Product boundary

Nickel evaluates declared configuration and policy values.
Molten still owns contract selection, defaults, decoding, authority, runtime effects, and release decisions.

A successful Nickel evaluation does not grant authority.
It does not prove that Molten policy is correct.
It does not prove runtime correctness or release readiness.

Diagnostics stay bounded by existing Molten error conversion and redaction paths.
Secret-like text must not become release evidence or unredacted operator output.
