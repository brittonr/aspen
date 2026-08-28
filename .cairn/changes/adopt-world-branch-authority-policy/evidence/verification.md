# World branch authority verification

## Status

Implementation is complete for every unblocked task.

Promotion-gated activation remains denied until `bind-world-promotion-to-effect-release` supplies effect-release reservation admission.
This change remains active, unsynced, unarchived, and unmerged.
Policy decisions and receipts do not reserve, dispatch, or authorize effects.

## Reviewed dependency cohort

Molten pins Basalt revision `89675cd4f585f837323c049e4a25f7b94c903038` for policy and runtime types.
That Basalt revision contains the reviewed UCAN import from revision `6f888f6c91a4ea26f0bd52b6486e6643c8f6d271`.
The Basalt UCAN snapshot manifest has BLAKE3 `a30addca944cb27578f2c9e8fbe8c133a6839ff9aabfa6f0151e436ff920a7be`.

Pueue task `3375` showed one `verified-logic 0.2.0` source identity.
The root and Basalt both use the copy from Basalt revision `89675cd4f585f837323c049e4a25f7b94c903038`.
`Cargo.lock` contains one BLAKE3 `1.8.5`, one Zeroize `1.9.0`, and one `verified-logic 0.2.0` package.

## Generated plans

Pueue task `3324` regenerated both unit2nix plans after the dependency lock changed.
No generated plan or lockfile was edited by hand.

- `build-plan.json`: 712 crates, four workspace members, 988 build units, and 1,018 test units.
- `release-policy-build-plan.json`: 253 crates, two workspace members, 322 build units, and two roots.

The release plan includes the `molten-release-policy` binary target.

## Rust verification

Pueue task `3330` passed these locked checks in the Nix development shell:

- `cargo fmt --all -- --check`
- workspace Clippy across all targets and features with `-D warnings`
- 286 `molten-core` tests and seven compile-fail doctests
- 1,374 `molten` library tests with one test thread
- 66 `molten` binary tests
- five `molten-release-policy` tests

The world-authority cases cover all closed Basalt modes and fail-closed mapping, realization, transfer, simulation, activation, and disclosure paths.

## Operator boundary

Pueue task `3331` passed the bounded CLI plan and denial smoke.
The plan receipt was `blake3:eba423b930c1ca3f112322f05758871b65daa40bde6410bb4caedb4c0d439cf2`.
The activation denial receipt was `blake3:234ee13232da23651ec526f7e94341e8c83f3c61fb11053f95f73d02bb56d47f`.

Activation without a runtime adapter failed with this diagnostic:

```text
error: invalid harness artifact: world branch activation requires an admitted runtime adapter
```

The receipt scan found no raw scope, bearer-token, or secret marker.

## Nix, Octet, and lifecycle verification

Pueue task `3332` built these focused checks:

- `world-authority-dependency-identity`
- `world-authority-octet-deny-all`
- `world-authority-schema-inventory`
- `release-dependency-profile`

The same task evaluated all flake outputs with `nix flake check --no-build`.
A full flake build is not claimed because the inherited `contract-export-drift-gate` failure remains outside this change.

Pueue task `3373` used the current Cairn source and the generated canonical policy.
Repository validation and the proposal, design, and tasks gates returned `PASS`.

## Bounded claims

These checks prove the recorded source, test, schema, policy, and receipt observations only.
They do not prove global revocation freshness, adapter correctness, future enforcement, deployment success, or release eligibility.
Unknown transfer or activation outcomes still require observation-first reconciliation.
Simulation-only authority still requires an exact deterministic adapter and never falls back to a live adapter.
