# Nickel toolchain cohort

r[impl molten.nickel_toolchain.evidence]

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

`nickel-lang-core` declares `links = "nix"`, so one copy serves the whole dependency graph.
`Cargo.toml` therefore patches `nickel-lang 2.2.0` and `nickel-lang-core 0.18.0` to nickel-eval's
owned import at revision `2d1fb2b59f373967cc69e13ec289759d9cd4df46`
(`https://seed.radicle.garden/z21miuQLqLwe1HkaMu2mEZy5fdG9F.git`). That import contains the same
reviewed crates.io archives, identified by SHA-256 and BLAKE3, plus tracked patches: the captured
resolver seam, the local-core wrapper selection, and the upstream imported-data closurization
repair. The captured seam is inert unless the `nickel-eval.captured-modules.v1` profile installs
its source cache. `nickel-lang-parser` and `nickel-lang-vector` remain crates.io packages.
nickel-eval's `docs/native-captured-source.md` records the archive, patch and vendor identities.

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
