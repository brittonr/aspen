# Verification evidence

## Baseline

The change branch started from canonical `origin/molten` commit `b5b1004cf4b80e7a133069780253c99728526682`.

Baseline `molten-core` and serial Molten library tests passed before the oracle implementation. No baseline failure blocked comparison.

The oracle remains optional. Default product composition does not expose `src/world_state_oracle`.

## Exact source and build cohort

The admitted source is `dolthub/doltlite` revision `10170ed82c1b12414db8d1b29d2fe9ea2a72fd88`.

The Nix package is:

`/nix/store/90c67z564pkwhwg0cz43gkmd6izbm654-molten-doltlite-oracle-10170ed8`

The package builds the non-amalgamation CLI and library with remotes and Vec1 disabled. It installs ten reviewed upstream harnesses.

- Binary BLAKE3: `blake3:019983d04bbbd689aec0faac418f99fa49f2f61e888bed8d9e0f34cfc8b3e08b`.
- Adapter BLAKE3: `blake3:a4b36a1a46cff61a2c4efb5fa4bbd26778467d65b9cc511197470607ac3e393a`.
- Adapter version: `molten-doltlite-oracle-v1`.
- Backend format: `doltlite-chunk-store-v12`.

Notice identities:

- `APACHE_LICENSE`: `blake3:a24e4e2958e399474e4b0913dde32c6be84630b6dcf153af7eae29779399eb2f`.
- `LICENSE.md`: `blake3:4f91d1a7d7b99eefb5c81ddb148446616d8260fc0c8113999cf2a48d3589267c`.

Contract identities:

- SQLite compatibility: `blake3:82d470f924e39e4e6eed5ce48095bcb30e682b15e076476a1caf847dac9ab664`.
- Concurrency: `blake3:9efcaf8c67d3b1d6c1e9eac578810bfd266bb2fb920344230299db143d6afcc8`.
- Storage format: `blake3:9a24814b1023720459092e2fc0126c09ad6af221b6ec876948d0c67c2bcb5452`.

`world-state-oracle-source` verifies the pin, notices, contracts, binary, adapter, build flags, feature gate, core purity, installed harnesses, and exact upstream selectors.

## Typed contracts

`config/world-state-oracle/source.ncl` records source, build, execution bounds, notices, contract inputs, and non-claims.

`config/world-state-oracle/ledger.ncl` has 18 closed rows:

- 8 compatible;
- 2 adapted;
- 7 intentional;
- 1 unsupported;
- 0 engine gaps.

The unsupported row is `multi-file-write`, with issue `dolthub/doltlite#storage-multi-file`.

Eight negative Nickel fixtures reject remote enablement, missing evidence, exception growth, identity overclaims, missing pins, notice drift, production enablement, and build-input drift.

The `world-state-oracle-profile` Nix check regenerated both JSON files and compared exact bytes. Every negative fixture failed export as required.

## Boundary and records

The pure core owns source admission, the compatibility ratchet, normalized observations, comparisons, and consumer-bound projections.

The root shell owns the test port, absolute paths, capability-rooted workspaces, cleared process environments, bounded streams, timeouts, process-group teardown, and typed infrastructure errors.

Four canonical Preserves labels are registered:

- `molten-semantic-state-oracle-source-v1`;
- `molten-semantic-state-oracle-observation-v1`;
- `molten-semantic-state-oracle-comparison-v1`;
- `molten-semantic-state-oracle-projection-v1`.

Projection tests bind separate Prolly and benchmark consumers. They remove backend roots and reject authority or correctness claims.

## Focused Rust tests

These focused checks passed:

- 4 `molten-core` oracle tests;
- 4 root oracle and record tests;
- 1 live Nix-built adapter integration test.

The live integration covered history independence, branch isolation, GC, exact reopen, detached reads, disabled remotes, rowid denial, custom-collation denial, and multi-file denial. Delegated cases returned explicit unsupported observations instead of success.

`cargo clippy --workspace --all-targets --all-features -- -D warnings` passed.

## Upstream harness evidence

The `world-state-oracle-upstream-harnesses` Nix check passed 636 checks with zero failures:

- detached head: 36;
- concurrent commit: 34;
- version-control concurrency: 37;
- multi-process concurrency: 54;
- multi-process GC: 101;
- GC tip survival: 45;
- corruption detection: 224;
- malformed commit deserialization: 4;
- catalog serialization determinism: 80;
- pending-state serialization: 21.

Expected lock and rollback diagnostics occurred in concurrency denial paths. They did not become success claims.

The separate storage format contract passed 23 checks. It covered version 12 reads and writes, version 11 and 13 rejection, bad-magic rejection, reopen, GC, and integrity.

These are bounded facts about the pinned cohort. They are not Molten authority or correctness proof.

## Full Rust suite

`cargo test --workspace --all-targets --all-features` passed after a test-only race repair:

- 1,409 Molten library tests;
- 74 Molten binary tests;
- 61 CLI harness tests;
- 12 content-replication tests;
- 6 profiling-boundary tests;
- 6 executable-extent tests;
- 3 simulation-boundary tests;
- 2 execution-boundary tests;
- 2 node-host facade tests;
- 8 native system-extension tests;
- 4 world-commit integration tests;
- 328 `molten-core` tests;
- 5 node-host boundary tests;
- 5 release-policy binary tests.

The first broad run exposed an existing timing race. One live child printed and exited before it read declared standard input. The test command now consumes the input before it prints. The focused all-feature test and the complete workspace suite then passed.

The live DoltLite integration remains ignored in the broad suite because it needs explicit Nix cohort variables. It passed separately.

## Octet

The first focused strict Octet run found 19 structural findings. The implementation then split its model, grouped compatibility rows, replaced long positional builders, renamed one predicate, and removed sentinel conversion fallbacks.

The final `world-state-oracle-octet-deny-all` check is clean:

- findings: 0;
- warnings: 0;
- errors: 0;
- config hash: `b3:b88b59cbc9928cdd25872bdb0cc9f29668a989c74498358048538a9aaca19742`;
- profile hash: `b3:8727cd28ead3211fc3143cfdd5bd817a75b94ab91c4d37ea7ad21b9f0b78f8d4`.

This focused result is separate from inherited broad-workspace Octet warning debt.

## Nix checks

These focused checks passed with local builders and secret-key files disabled:

- `world-state-oracle-profile`;
- `world-state-oracle-source`;
- `world-state-oracle-schema-inventory`;
- `world-state-oracle-format-contract`;
- `world-state-oracle-upstream-harnesses`;
- `world-state-oracle-octet-deny-all`.

`nix flake check --no-build` evaluated every package, check, app, shell, and formatter.

The known full-flake inherited Tracey path blocker remains outside this change.

## Generated plans

The pinned unit2nix tool generated each plan twice with identical bytes.

- `build-plan.json`: 713 crates, 4 workspace members, 989 build units, and 1,019 test units.
- `release-policy-build-plan.json`: 254 crates, 2 workspace members, 323 build units, and 2 roots.
- The release plan contains the `molten-release-policy` binary and has no dev-dependency projection.
- Cargo lock SHA-256: `4831149e509872705a47b22aeb5e48e06a2eb7b15524fc09ee7cbf766ffcf935`.
- Main-plan BLAKE3: `6b41f9aba665033823eebafac0db2c3526e526a6350a038af8e330a0168a7d63`.
- Release-plan BLAKE3: `f9093f1fcbff1683b2a44da68dd0b986e6fe39b23439efc4c25101b45a54c5d3`.

SHA-256 appears only because unit2nix defines that interoperability field.

One transient Radicle seed prefetch produced a different Choregraph NAR hash for the same revision. `crate-hashes.json` now binds the already accepted canonical hash `0rifkbnc5vsddmlq7slsx7s1jv4blfd0qdk5sgyfjkfwbc21ksll`. Both subsequent generations were byte-identical.

## Claim boundary

Oracle agreement is evidence for one case and one exact cohort. It does not prove Molten correctness, root-format equality, complete-world atomicity, durable conflict safety, production readiness, or release eligibility.

Oracle records never authorize mutation, cleanup, activation, deletion, promotion, effect dispatch, or release.

## Lifecycle

Cairn validation passed under policy hash `8280151b7a53822eed460149ecb600bf11418d7463c935e0742009b334f4e7dd`.

Gate receipts:

- proposal input `1a093bdae978197a92313d2104b56de02b515c4ad408ca2bda8bb758ccc0e3a4`, receipt `f69c8097a8b8c2d00a6610b6a94164418123ec8e0f95968e7438db80e8f93cc4`;
- design input `d27aea9e42dfcb40664cf3ab47b421d230186d812014b7e4ca55dfa8d27fb5af`, receipt `544cca729203c49961d17ed8f40e837c5843ec22ee660013d1dd4d62ab159b34`;
- tasks input `26feebfb046df28a6fdf887f2ade33c2643883863760f4a3146344b0218873e1`, receipt `473516f91d06ace5829d26295491dfc2a6f4ff30225ea3a810bfb20e70f608aa`.

Sync dry-run was unblocked. Its plan hash was `5eb93388b3eafcea4d1bb0f19d176fe2fc96d68149a3f7bbafeed5b0e7e03623` and receipt was `824c7c1452cd8a0e7468e42576a62357744a001eb0cd2c577c18f81431ee3cc1`.

Executed sync receipt `34d9835e3800f18a797180f058945ecd7627dec0057fb6038412ea2f87c29c7f` produced accepted world-commit spec hash `66a9eab8f5fa2bd9e34e7a570ade56a20d0d4c25fbe0c5778d91132394207f94`.

All six `molten.world_state_oracle.*` requirements are present in `.cairn/specs/world-commit/spec.md`.

Archive dry-run was unblocked. Its plan hash was `6eb2eea8e0528f9d56a84d45958f18ba4b716ba82a41c616b0b197f09eca3039` and receipt was `221d349b6fba6e9c3845389000d2b9e79b555171c527eaf7b067b729ac825b6f`.

The executed archive receipt is recorded after mutation.
