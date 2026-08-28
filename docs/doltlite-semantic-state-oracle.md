# DoltLite semantic-state oracle

Molten uses DoltLite as an optional test oracle. The oracle checks selected semantic-state behavior through an implementation that does not share Molten's native state-map code.

The oracle is evidence only. It does not own product state, authority, policy, retention, effects, or release decisions.

## Reviewed source

The Nix input pins this source:

- Repository: `https://github.com/dolthub/doltlite`
- Revision: `10170ed82c1b12414db8d1b29d2fe9ea2a72fd88`
- Adapter version: `molten-doltlite-oracle-v1`
- Backend format: `doltlite-chunk-store-v12`
- Binary BLAKE3: `blake3:019983d04bbbd689aec0faac418f99fa49f2f61e888bed8d9e0f34cfc8b3e08b`
- Adapter BLAKE3: `blake3:a4b36a1a46cff61a2c4efb5fa4bbd26778467d65b9cc511197470607ac3e393a`

The build uses the full pinned checkout. It builds the non-amalgamation CLI and library with `stdenv.cc`, `gnumake`, and `zlib`.

The build sets these values:

- `DOLTLITE_ENABLE_REMOTES=0`
- `DOLTLITE_VEC1=0`
- `DOLTLITE_VERSION=10170ed8`

The root Cargo feature `doltlite-oracle` exposes the test shell. Default product builds do not expose `src/world_state_oracle`.

## License and notice boundary

The package preserves these source files:

| File | Boundary | BLAKE3 |
| --- | --- | --- |
| `APACHE_LICENSE` | DoltLite Apache-2.0 code | `a24e4e2958e399474e4b0913dde32c6be84630b6dcf153af7eae29779399eb2f` |
| `LICENSE.md` | SQLite public-domain statement and project notices | `4f91d1a7d7b99eefb5c81ddb148446616d8260fc0c8113999cf2a48d3589267c` |

Vendored code keeps its own terms. This pilot does not change any public license.

## Contract inputs

The package preserves three upstream contract tables:

| Contract | BLAKE3 |
| --- | --- |
| `test/sqlite_compatibility_contract.tsv` | `82d470f924e39e4e6eed5ce48095bcb30e682b15e076476a1caf847dac9ab664` |
| `test/concurrency_contract.tsv` | `9efcaf8c67d3b1d6c1e9eac578810bfd266bb2fb920344230299db143d6afcc8` |
| `test/storage_format_contract.tsv` | `9a24814b1023720459092e2fc0126c09ad6af221b6ec876948d0c67c2bcb5452` |

`config/world-state-oracle/source.ncl` records the source and execution profile. `config/world-state-oracle/ledger.ncl` records the closed compatibility rows.

The generated JSON files are review artifacts. The Nix profile check exports each Nickel source again and compares the bytes.

## Ownership boundary

`molten-core` owns pure source admission, compatibility, observations, comparisons, and consumer projections. It has no SQLite handles, DoltLite types, file paths, process state, or hidden current-branch state.

The root test shell owns:

- the `SemanticStateOracle` port;
- disposable capability-rooted directories;
- absolute executable and workspace paths;
- a cleared process environment;
- bounded standard input, output, and error streams;
- the timeout and process-group teardown;
- parsing of DoltLite output into Molten rows and outcomes.

DoltLite object and root values stay in `backend_root`. They are backend-local evidence. A comparison ignores their spelling.

## Resource bounds

Each live adapter request uses these bounds:

| Resource | Bound |
| --- | ---: |
| Process timeout | 10,000 ms |
| Teardown timeout | 1,000 ms |
| Standard input | 65,536 bytes |
| Standard output | 262,144 bytes |
| Standard error | 65,536 bytes |
| Semantic rows | 256 |
| Key size | 256 bytes |
| Value size | 4,096 bytes |
| Diagnostics | 32 |

An unknown process outcome is an infrastructure error. It does not imply success and does not permit a blind retry.

## Normalized cases

The live adapter covers these positive cases:

- history-independent state under different insertion orders;
- branch-visible state isolation;
- reader-safe garbage collection;
- exact-format reopen;
- detached read-only state;
- stable key ordering with explicit primary keys.

The live adapter covers these negative cases:

- remotes are disabled;
- rowid identity is rejected;
- custom collation is rejected;
- multi-file file-backed writes are unsupported.

The pinned upstream harnesses add bounded evidence for:

- stale snapshot and competing-writer conflict handling in `multi_process_test`;
- concurrent commit contention in `concurrent_commit_test` and `vc_concurrency_test`;
- reader and writer coordination during GC in `multi_process_gc_test`;
- reachable-tip survival in `gc_tip_survival_test`;
- tampered storage in `corruption_test`;
- malformed commit images in `commit_deserialize_test`;
- deterministic catalog serialization in `catalog_serialize_determinism_test`;
- native image round trips in `serialize_pending_test`;
- detached reads in `detached_head_test`.

The source gate binds these exact upstream selectors:

| Case | Selector |
| --- | --- |
| stale snapshot | `test/concurrent_commit_test.c#pin_commit_rejected` |
| competing writer | `test/multi_process_test.c#mp_conflict_detected` |
| live reader during GC | `test/multi_process_gc_test.c#reader_iter_saw_all_rows` |
| detached read | `test/detached_head_test.c#attached tag readonly` |
| storage tamper | `test/corruption_test.c#tampered_wal_offset_detected` |
| wrong format | `test/corruption_test.c#bad_version_detected` |
| malformed commit image | `test/commit_deserialize_test.c#trunc_before_email_len` |
| serialization | `test/serialize_pending_test.c#fresh-db image deserializes` |
| deterministic serialization | `test/catalog_serialize_determinism_test.c#reopen_bytes_match` |
| version skew | `test/storage_format_contract_test.sh#skew_version_13_notadb` |

These harnesses test the pinned cohort only. They do not make a general SQLite or Dolt promise.

## Compatibility ledger

The standard ledger has 18 rows:

- 8 compatible;
- 2 adapted;
- 7 intentional;
- 1 unsupported;
- 0 engine gaps.

The maxima are part of the contract. A new exception fails the profile unless a reviewer changes the typed policy.

The unsupported row is `multi-file-write`. It tracks `dolthub/doltlite#storage-multi-file`.

Molten keeps these intentional differences:

- complete-world atomicity;
- durable typed conflicts;
- typed merge policy;
- authority admission;
- effect reservation and dispatch;
- retention and deletion policy;
- stack-global BLAKE3 identity domains.

## Differential evidence

An observation contains ordered semantic rows, a branch, an outcome, exact source and adapter references, and optional backend-local evidence.

A comparison can report agreement, divergence, or unsupported behavior. Agreement applies only to that case and input.

`project_oracle_evidence` emits separate consumer-bound records for:

- `pilot-prolly-semantic-state-map`;
- `benchmark-world-commit-sharing-and-retention`.

The projection removes the backend root. It sets authority and correctness claims to false. A crossed observation/comparison link or any overclaim fails validation.

Canonical Preserves records use these labels:

- `molten-semantic-state-oracle-source-v1`;
- `molten-semantic-state-oracle-observation-v1`;
- `molten-semantic-state-oracle-comparison-v1`;
- `molten-semantic-state-oracle-projection-v1`.

## Production non-goals

This pilot does not establish:

- a production Molten database;
- a default runtime dependency;
- SQLite compatibility;
- Molten correctness;
- complete-world atomicity;
- durable conflict safety;
- remote or network safety;
- global identity equality;
- production readiness;
- release eligibility.

A receipt records a bounded observation. It never authorizes mutation, cleanup, activation, deletion, effect dispatch, promotion, or release.
