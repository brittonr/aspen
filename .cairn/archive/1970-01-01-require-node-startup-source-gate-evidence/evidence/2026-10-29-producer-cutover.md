# Exact startup producer integration — 2026-10-29

## Executed baseline, before cutover

The integration owner ran the real existing CLI test on clean main `d7e435e711f7f58167900ce7419121390b0842d3` before authorizing startup behavior edits:

`cargo test --locked -p molten --test cliharness cli_node_init_run_status_and_stop_write_receipts -- --exact --nocapture`

Result reported by the integration owner: **1 passed, 61 filtered**, test execution 0.21 seconds; transcript `artifact://5942`. This reproduces normal artifactless startup acceptance. It is not a real-artifact positive control.

## Implemented boundary, not yet executed acceptance

The new prerequisite is `.cairn/changes/produce-exact-node-startup-binding/`. Its product-owned Nix profile constructs candidate and gate builds from the same immutable source, evaluates real strict gate artifacts and measures actual ELF/source/toolchain/build-closure/policy/profile/cohort bytes. Its measurement script does not sign or authorize anything. Independently selected full-key Nix output signatures authenticate the reviewed producer claim; content-addressed and implicit local signing trust are rejected.

Normal daemon startup now reads operator-provisioned `source-gate-trust.json`, authenticates exact Nix output content, remeasures the actual executing ELF and retained artifacts, and performs bounded pure candidate admission before restart preparation or lifecycle writes. The validation receipt has a versioned real-binding schema. Explicit cfg(test) constructors use a separate fixture-validation schema; normal wrappers cannot select them. Existing daemon domain tests were migrated to those explicit fixture constructors. Real CLI positive tests require the independently provisioned `MOLTEN_TEST_SOURCE_TRUST` cohort and execute its actual signed output binary.

The read-only public source gate is `node_daemon::admit_startup_source_gate_with_root(&DirectoryRoot)`. It returns an opaque source-only token, not hosting, membership, endpoint, transport, timer, production or activation authority. Normal runtime compatibility composition is available through `node_daemon::start_runtime_with_source_root` and uses the same real loader.

## Pending actual acceptance and authority

No post-cutover checks were run by the worker, per the integration owner's no-mid-flight-check contract. Pure, adapter and real CLI smoke cases have been authored; authored cases are not executed evidence. The integration owner must run actual same-source Octet production, build and independently sign the real output, execute the normal-startup positive and missing/stale/tamper/wrong-candidate/synthetic/read-error controls, and repeat required gates before checking completion tasks or archiving.

`scripts/smoke-startup-source-binding.py` requires real independently signed positive and deliberately tampered experimental outputs. A separately provisioned explicitly experimental authority can exercise the actual mechanism, but does not constitute production operator approval. Existing host Nix public/signing configuration was inspected without reading any secret key; its existence does not authorize a new producer profile. No fake gate status, self-trusted signature, fixture-based positive, production admission flip or compiler-correctness claim is recorded here.
