# Aspen object-storage qualification

This is a scoped qualification recipe for issues #8, #13 and #14, not an execution receipt. Aspen retains internal `molten` crate, CLI and schema names. No commands below were run as part of this source change. A directory on tmpfs, an SSD flag, or an ignored test definition is not NVMe evidence.

## Profile and recovery admission

The opt-in `content-store-object-storage` feature uses the existing thin SigV4 HTTP client, not an SDK, worker pool, discovery service or multipart uploader. A simple PUT is at most 64 MiB. `execute_object_put` and `fetch_object_chunks` drive the existing pure transition core; the publication helper remains a verified streaming upload convenience, not a canonical operation receipt. Only confirmed PUT acknowledgements yield `Durable`; GET yields `Verified`, never durability merely because it was cached.

The local tier is an exclusively owned `Scope`, retained across descendant effects. Do not point independent clients or external writers at the same tier root. `from_recovery_scope` validates a `ContentTierRecoveryBudget` before opening inventory. `max_local_tier_bytes` is independent of operation bytes, memory and remote inventory size. There are at most 4096 entries; repeat puts do not consume capacity twice. Writes reserve capacity before effects, so failed cache writes cannot free capacity silently. Reopen rejects unsupported/non-regular entries and over-cap inventory. Cache repair/eviction is local-only and does not authorize remote deletion.

The source Nickel policy's worked example is 64 GiB of hot chunks, re-fetched and re-verified from object storage, with a **declared minimum sustained verified rate** of 1 GiB/s and a 64-second transfer budget: `64 GiB / (1 GiB/s) = 64 s`. This conditional bound assumes the declared minimum is actually maintained, including verification. Endpoint outages, stalls, queueing and device recovery require independent budgets; this is not an unconditional MTTR guarantee. Growing remote inventory cannot increase the hot-tier cap. At cap, a new cold GET is verified and returned remote-only, and a PUT can complete after remote confirmation without local growth. There is no automatic eviction. TigerBeetle's cited 128 TiB, 2.9-hour and fleet CPU numbers are unreproduced literature, not Aspen targets or results.

Each client has one remote-request slot, checked with `try_acquire` before dispatch; body consumption holds that slot. Idle pooling, redirects, proxies and automatic retries are disabled. The endpoint must be numeric and is fixed at construction; no request-path OS DNS exists. Plain `http://` is admitted only for a loopback endpoint such as the disposable fixture; any other endpoint must use `https://`, because SigV4 protects the credential but not the payload. Credentials are fixed at admission with **no refresh operation** and therefore no IdP thread/call/refresh interval. Operators must separately admit a replacement client when credentials expire. Configuration strings, connect/request duration (at most 30 seconds), and response buffering are bounded; a body without `Content-Length` is read chunk by chunk and stops at the bound. The redacted `remote_policy()` reports this policy without locators or credential bytes.

## Reproducible local endpoint

The disposable Linux x86-64 fixture pins SeaweedFS 4.53 and verifies the published SHA256 before extracting `weed`:

- URL: <https://github.com/seaweedfs/seaweedfs/releases/download/4.53/linux_amd64.tar.gz>
- SHA256: `d1a7dec9f48d8b33895a0ea8951971defc2dc5312906dcc76fa5a5f74a6ee90d`
- Upstream asset metadata: <https://api.github.com/repos/seaweedfs/seaweedfs/releases/assets/724882169>

Select an empty task-owned work directory. Run the server in a managed foreground session, then provision its disposable bucket in another session. No production endpoint, credentials, lifecycle policy or inventory is authorized by this fixture.

```sh
bash docs/content-store-adapter/endpoint-fixture.sh install "$ENDPOINT_WORK"
bash docs/content-store-adapter/endpoint-fixture.sh serve "$ENDPOINT_WORK"
# Another session, after the managed server starts:
bash docs/content-store-adapter/endpoint-fixture.sh bucket "$ENDPOINT_WORK"
export MOLTEN_S3_ENDPOINT=http://127.0.0.1:28433
export MOLTEN_S3_BUCKET=molten-live
mkdir -p "$RECEIPT_DIR"
export MOLTEN_CONTENT_RECEIPT_DIR="$RECEIPT_DIR"
```

The CI workflow selects the explicit `object-storage-ci` Nix shell with the existing pinned Rust compiler, compiler/linker support, Git and Python. It uses this checksum-pinned endpoint fixture, the unchanged dependency guard, focused mock/core arm and local endpoint command arm, and retains actual canonical artifacts. It does not execute or qualify the unrelated proof producers in the default development shell. Wiring the workflow is not a recorded passing CI run.

## Parent verification commands

Run these only after integration edits have settled, in the reviewed development shell. Do not describe unexecuted source fixtures as passed.

```sh
python3 docs/content-store-adapter/dependency-guard.py
cargo test -p molten-core content_store_adapter::
cargo test -p molten --lib --features content-store-object-storage content_store_adapter::
# The live roundtrip, corrupt-remote parity and full canonical receipt fixtures:
cargo test -p molten --lib --features content-store-object-storage content_store_adapter::live_object::live -- --ignored --test-threads=1
cargo test -p molten --lib --features content-store-object-storage content_store_adapter::live_object::completion_tests::live_command_conformance_emits_canonical_receipt -- --ignored --exact
cargo test -p molten --lib --features content-store-object-storage content_store_adapter::live_cache::tests::live_endpoint_roundtrip -- --ignored
# #13: model-only canonical receipt, no S3 required:
cargo test -p molten --lib simulated_put_classifies_throttle_torn_put_and_multipart_abort -- --nocapture
nickel export docs/content-store-adapter/profile.ncl
# This negative fixture must fail export:
nickel export docs/content-store-adapter/fixtures/negative/recovery-budget-exceeded.ncl
```

The live conformance fixture executes admitted PUT, cold GET, HEAD availability, protect audit, and protected/unprotected deletion denials without synthesizing deletion admission. It then deliberately deletes its own test object through a separate fault client to model an out-of-band lifecycle rule and asserts that the protected-remote audit fails despite the retained valid local copy. This destructive fault arm refuses any endpoint other than `http://127.0.0.1:28433` and bucket other than `molten-live`; use only the disposable fixture. It records 128 forced remote reads and 128 local hits independently. `object-conformance.preserves` retains put/get state and command digests, local residency/capacity, declared recovery budget, policy, lifecycle-loss detection and separate p50/p99 microseconds. These finite-run observations do not support a speedup claim. Regular command tests independently cover no-request local hits, cap-exceeding verified remote-only reads and protected-remote loss. Canonical deletion admission is separate from the local protection veto; provider lifecycle rules are outside the adapter's control and must not target protected inventory.

`put-faults.preserves` records actual simulated provider-throttle/torn-put/multipart-abort terminals and canonical fixture command digests after execution. HTTP mock tests exercise the same live client and command transition path for corrupt, truncated, missing, disconnect, timeout, throttle and rejected/torn PUT observations. Multipart abort is a deterministic model fault, **not** evidence that a multipart wire uploader exists. Get-path verified progress and put-side mutation uncertainty remain distinct; uncertain PUT state cannot silently resume. No ambiguous PUT is retried automatically.

The direct dependency allowlist is `object-storage-dependency-allowlist.tsv`. Root integration exact-pins those optional dependencies and keeps the feature absent from defaults. The guard rejects direct `aws-sdk*`, `aws-config` and `object_store` without a reviewed ADR/policy change. The bounds slice added no dependencies. The [historical comparison](../../.cairn/archive/2026-11-15-complete-object-storage-bounds/evidence/dependency-cohorts-2026-11-16.json) retains the original client-introduction and subsequent feature-gating lock inventories, exact commits/blobs, BLAKE3 inputs and changed package identities:

| Historical change | Before packages | After packages | Delta |
| --- | ---: | ---: | ---: |
| Original bounded SigV4 client | 997 | 1005 | +8 |
| Subsequent optional feature gating | 1005 | 1005 | 0 |

These are historical whole-lock inventories, not a current enabled/disabled feature resolution or attribution of every package change to object storage. No lock was regenerated or edited to produce the comparison.

## Actual NVMe-path inputs

Supply both an existing writable path on the intended NVMe filesystem and its **independently selected actual block-device sysfs path** (partition included). The ignored fixture checks the path's `st_dev` against that exact sysfs device, checks NVMe ancestry and non-rotation, and runs capacity/reopen verification in a dedicated child directory on that filesystem. Overlay/tmpfs/device-mapper without an exact exposed NVMe mapping fails closed; do not substitute a generic temp directory.

```sh
export MOLTEN_NVME_ROOT=/operator/selected/nvme/mount
export MOLTEN_NVME_SYSFS=/sys/class/block/nvme0n1p1 # replace with actual independently selected device
export MOLTEN_CONTENT_RECEIPT_DIR="$RECEIPT_DIR"
cargo test -p molten --lib --features content-store-object-storage qualify_actual_nvme_path_capacity_and_reopen -- --ignored --nocapture
```

`nvme-path.preserves` binds observed device major/minor and a BLAKE3 ref of the actual sysfs device together with the tested cap. It proves scoped path/capacity/reopen behavior only, not controller durability, performance or machine-loss recovery. The [parent execution record](../../.cairn/archive/2026-11-15-complete-object-storage-bounds/tasks.md#parent-execution-2026-11-13) records the completed local S3/cache and actual NVMe path controls. The recovered [original packed NVMe receipt](../../.cairn/archive/2026-11-15-complete-object-storage-bounds/evidence/nvme-path.preserves) is 173 bytes with BLAKE3 `f62fc0061837fef1f9070e12e3de0e88c17e9fb26e85737b2bfcee531f7de09b`; its [recovery record](../../.cairn/archive/2026-11-15-complete-object-storage-bounds/evidence/nvme-recovery-2026-11-16.json) does not claim a new current-source execution.

Hosted local-S3 runs [43810362842](https://github.com/brittonr/aspen/actions/runs/43810362842) at `fcbf60a3d` and [43813756730](https://github.com/brittonr/aspen/actions/runs/43813756730) at `f663d2f47` passed. The latter retains 47 passing test names and 14 original packed canonical artifacts, including conformance and simulated put-fault receipts, in the [exact cohort capture](../../.cairn/archive/2026-11-16-admit-anonymous-runtime-sources/evidence/hosted-ci-20261116/observations.json). This satisfies the local endpoint CI observation, not NVMe performance, cloud-provider behavior, physical power loss, machine loss, sustained throughput or later-source qualification.

The earlier [storage run 43693265859](https://github.com/brittonr/aspen/actions/runs/43693265859) failed during default-shell source resolution before the guard or conformance tests. Its endpoint-log-only artifact remains failed-attempt history; it is not substituted for the later canonical observations. The [source/tool boundary](../reproducible-dependencies.md) and default/full-flake acceptance remain separate.
