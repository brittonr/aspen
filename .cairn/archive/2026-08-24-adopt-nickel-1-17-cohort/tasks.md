# Tasks: Adopt the Nickel 1.17 evaluator cohort

## Cohort update

- [x] [serial] Pin `nickel-lang 2.2.0` and its `nickel-lang-core 0.18.0` cohort. r[molten.nickel_toolchain.cohort]
- [x] [serial] Add an exact Nickel CLI `1.17.0` source input at commit `1320a983e6c3d1e2fb53dd2464b084b4903b1426`. r[molten.nickel_toolchain.cohort]
- [x] [serial] Regenerate Cargo and Nix lockfiles only through Cargo and Nix commands. r[molten.nickel_toolchain.cohort]
- [x] [parallel] Add a guard that rejects old, mixed, floating, or ambient Nickel dependencies. r[molten.nickel_toolchain.cohort]

## Boundary and compatibility

- [x] [serial] Adapt embedded evaluator errors and values at the existing shell boundary. r[molten.nickel_toolchain.boundary]
- [x] [parallel] Prove valid evaluation does not bypass Molten policy or authority checks. r[molten.nickel_toolchain.boundary]
- [x] [parallel] Run representative valid policy, configuration, receipt, and runtime-profile fixtures. r[molten.nickel_toolchain.compatibility]
- [x] [parallel] Add malformed, missing-import, contract, bound, unknown-field, and redaction negative fixtures. r[molten.nickel_toolchain.compatibility]

## Evidence and validation

- [x] [serial] Record crate versions, CLI version, upstream commit, compatibility results, and non-claims. r[molten.nickel_toolchain.evidence]
- [x] [parallel] Add stale-cohort, identity-mismatch, and overclaim evidence failures. r[molten.nickel_toolchain.evidence]
- [x] [serial] Run focused tests, policy checks, formatting, Clippy, Cairn gates, and relevant Nix checks. r[molten.nickel_toolchain.validation]
