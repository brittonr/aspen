# Runtime Host Readiness

This page records Aspen's current runtime-host evidence boundary for operator and reviewer use. It is a historical acceptance artifact, not a live status endpoint: rerun the gated checks at the commit you want to cite before making a new release/readiness claim.

## Current proven boundary

Aspen currently has five runtime-host rows with executable evidence:

1. Guest-backed microVM CI execution evidence for the archived OpenSpec runtime-host matrix row:

```text
OpenSpec change: openspec/changes/archive/2026-05-07-add-runtime-host-e2e-matrix
matrix row: runtime-host-microvm-ci-vm
target: checks.x86_64-linux.vm-snapshot-e2e-test
proof level: aspen-spawned-execution
```

2. Product-path WASM job execution evidence for the archived OpenSpec promotion row:

```text
OpenSpec change: openspec/changes/archive/2026-05-08-promote-wasm-runtime-host-e2e
matrix row: runtime-host-wasm-product-path
target: cargo test -p aspen-jobs --test wasm_product_path_test --features plugins-wasm
proof level: aspen-spawned-execution
```

The WASM proof submits a blob-backed `wasm_component` job through `JobManager`, runs it through `WorkerPool` orchestration, executes a declared `aspen:runtime-host/wasm-v1` fixture export, and checks the product-visible receipt marker `ASPEN_WASM_RUNTIME_HOST_EXECUTED`.

The guardrail marker `ASPEN_WASM_RUNTIME_HOST_PRODUCT_PATH_GUARD` separates negative coverage from execution evidence: runtime-core-only admission tests and plugin install/reload plumbing remain useful, but neither satisfies the WASM runtime-host row by itself.

3. Product-path Hyperlight job execution evidence for the archived OpenSpec promotion row:

```text
OpenSpec change: openspec/changes/archive/2026-05-08-promote-hyperlight-runtime-host-e2e
matrix row: runtime-host-hyperlight-product-path
target: cargo test -p aspen-jobs --test hyperlight_product_path_test --features plugins-vm -- --ignored --nocapture
proof level: aspen-spawned-execution
```

The Hyperlight proof submits a blob-backed `vm_execute` job through `JobManager`, runs it through `WorkerPool` orchestration, executes a declared `aspen:runtime-host/hyperlight-v1` guest entrypoint, and checks the product-visible receipt marker `ASPEN_HYPERLIGHT_RUNTIME_HOST_EXECUTED`.

The guardrail marker `ASPEN_HYPERLIGHT_RUNTIME_HOST_PRODUCT_PATH_GUARD` separates negative coverage from execution evidence: worker construction, payload serialization, package builds, ignored/manual examples, and direct worker-only calls remain useful, but none satisfies the Hyperlight runtime-host row by itself.

4. Product-path OCI lowering job execution evidence for the archived OpenSpec promotion row:

```text
OpenSpec change: openspec/changes/archive/2026-05-08-promote-oci-lowering-runtime-host-e2e
matrix row: runtime-host-oci-lowering-product-path
target: cargo test -p aspen-jobs --test oci_lowering_product_path_test --features plugins-wasm
proof level: aspen-spawned-execution
```

The OCI lowering proof admits an immutable `sha256:` OCI source identity, derives a declared `aspen:runtime-host/wasm-v1` target artifact, submits that derived artifact as a blob-backed WASM job through `JobManager`, runs it through `WorkerPool` orchestration, and checks the product-visible receipt marker `ASPEN_OCI_LOWERING_RUNTIME_HOST_EXECUTED`.

The guardrail marker `ASPEN_OCI_LOWERING_RUNTIME_HOST_PRODUCT_PATH_GUARD` separates negative coverage from execution evidence: raw containers, mutable tags, admission/model checks, lowering/build-only checks, and helper-only paths remain useful guardrails, but none satisfies the OCI lowering runtime-host row without the product-path target execution receipt.

5. Gated product-path Hermit/Uhyve job execution evidence for the archived OpenSpec promotion row:

```text
OpenSpec change: openspec/changes/archive/2026-05-08-promote-hermit-uhyve-runtime-host-e2e
matrix row: runtime-host-hermit-uhyve-product-path
target: ASPEN_UHYVE=<uhyve> ASPEN_HERMIT_UHYVE_IMAGE=<x86_64-unknown-hermit image> cargo test -p aspen-jobs --test hermit_uhyve_product_path_test --features plugins-vm hermit_uhyve_executes_declared_fixture_through_product_orchestration -- --ignored --nocapture
preferred fixture: nix build .#uhyve .#hermit-uhyve-marker plus checks.x86_64-linux.hermit-uhyve-marker-contract
proof level: aspen-spawned-execution
```

The Hermit/Uhyve proof submits a blob-backed `hermit_uhyve` job through `JobManager`, runs it through `WorkerPool` orchestration with `HermitUhyveWorker`, executes a real `x86_64-unknown-hermit` image using real Uhyve, and checks the product-visible receipt marker `ASPEN_HERMIT_UHYVE_RUNTIME_HOST_EXECUTED`. Prefer the source-built `.#hermit-uhyve-marker` fixture and `.#uhyve` package for reruns; the marker package build and metadata contract are reproducibility prerequisites only, not runtime-host proof by themselves. The fixture metadata pins `proof_boundary = fixture-build-is-not-runtime-host-proof`.

The guardrail marker `ASPEN_HERMIT_UHYVE_RUNTIME_HOST_PRODUCT_PATH_GUARD` separates negative coverage from execution evidence: fake-Uhyve tests, direct Uhyve shell commands, package builds, successful exits without the expected marker, skipped/ignored tests not run on a capable host, and direct worker-only calls remain useful guardrails, but none satisfies the Hermit/Uhyve runtime-host row without the product-path job receipt.

The microVM proof is stronger than inventory registration. The gated check boots an Aspen node, creates a Cloud Hypervisor golden snapshot, restores guest VMs from that snapshot, registers a guest worker with the Aspen cluster, submits real `ci_vm` jobs through `aspen-cli`, and waits for job completion.

## Latest accepted evidence

Latest accepted runtime-host microVM evidence on `main`:

```text
commit: 860a1d6c8 Trim snapshot VM E2E diagnostics
implementation commit: 56006f9cc Prove snapshot VM CI execution
gated check: nix build --impure .#checks.x86_64-linux.vm-snapshot-e2e-test --no-link -L --option sandbox false
result: passed
derivation: /nix/store/x5yz9rni6c269sq4lrc0ka5fzdjfx7zv-vm-test-run-vm-snapshot-e2e.drv
evidence log: .agent/evidence/runtime-host-e2e/vm-snapshot-e2e-trimmed-pass-20260508.log
log length: 928 lines
duration: test script finished in 96.45s
```

The earlier full proof log from the implementation commit remains useful for forensic comparison:

```text
evidence log: .agent/evidence/runtime-host-e2e/vm-snapshot-e2e-pass-20260508.log
log length: 1398 lines
derivation: /nix/store/3dd19akflhxgvp4rz2yf2zfkzi0m1yy9-vm-test-run-vm-snapshot-e2e.drv
```

## Proof markers to require

### MicroVM CI

A passing microVM check is only runtime-host evidence when the log includes these proof markers:

- Guest network configuration from the direct-boot microVM path, for example `ASPEN_CI_NET_CONFIG ip=10.200.0.10 dev=eth0`.
- Guest worker registration, for example `worker registered with cluster` in the guest serial log and `Snapshot VM worker registered with cluster` from the host test script.
- Real job execution through Aspen CI, specifically `CI job completed via snapshot-restored VM`.
- Snapshot restore reuse evidence, such as `Second job (snapshot-restored VM): <seconds>s`.
- Concurrent restored-VM stress evidence, including `All stress test jobs completed` and the COW efficiency summary.

### WASM job runtime

A passing WASM check is only runtime-host evidence when the test and receipt include these markers:

- The harness row `runtime-host-wasm-product-path` is `e2e-registered`, not `metadata-only`.
- The target is `cargo test -p aspen-jobs --test wasm_product_path_test --features plugins-wasm`.
- The successful job result contains `ASPEN_WASM_RUNTIME_HOST_EXECUTED`, `aspen:runtime-host/wasm-v1`, and `entrypoint: execute`.
- The negative guardrail contains `ASPEN_WASM_RUNTIME_HOST_PRODUCT_PATH_GUARD` and proves invalid WASM reaches the product worker path before failure.

### Hyperlight job runtime

A passing Hyperlight check is only runtime-host evidence when the test and receipt include these markers:

- The harness row `runtime-host-hyperlight-product-path` is `e2e-registered`, not `metadata-only`.
- The target is `cargo test -p aspen-jobs --test hyperlight_product_path_test --features plugins-vm -- --ignored --nocapture`.
- The successful job result contains `ASPEN_HYPERLIGHT_RUNTIME_HOST_EXECUTED`, `aspen:runtime-host/hyperlight-v1`, and `entrypoint: execute`.
- The successful job records at least one `WorkerPool` execution attempt after submitting a blob-backed `vm_execute` payload.
- The negative guardrail contains `ASPEN_HYPERLIGHT_RUNTIME_HOST_PRODUCT_PATH_GUARD` and proves invalid Hyperlight bytes reach the product worker path before failure.

### OCI lowering job runtime

A passing OCI lowering check is only runtime-host evidence when the test and receipt include these markers:

- The harness row `runtime-host-oci-lowering-product-path` is `e2e-registered`, not `metadata-only`.
- The target is `cargo test -p aspen-jobs --test oci_lowering_product_path_test --features plugins-wasm`.
- The lowering input uses an immutable `sha256:` OCI source identity and not a mutable tag alone.
- The lowering output declares a derived isolated target artifact with `aspen:runtime-host/wasm-v1` and `entrypoint: execute`.
- The successful product receipt contains `ASPEN_OCI_LOWERING_RUNTIME_HOST_EXECUTED`, the source OCI digest, the derived target artifact hash, and the target worker marker `ASPEN_WASM_RUNTIME_HOST_EXECUTED`.
- The successful job records at least one `WorkerPool` execution attempt after submitting the blob-backed derived artifact.
- The negative guardrail contains `ASPEN_OCI_LOWERING_RUNTIME_HOST_PRODUCT_PATH_GUARD` and proves raw containers, mutable tags, model-only lowering receipts, and invalid derived target artifacts are not execution evidence.

### Hermit/Uhyve job runtime

A passing Hermit/Uhyve check is only runtime-host evidence when the test and receipt include these markers:

- The harness row `runtime-host-hermit-uhyve-product-path` is `e2e-registered`, not `metadata-only`.
- The target is `ASPEN_UHYVE=<uhyve> ASPEN_HERMIT_UHYVE_IMAGE=<x86_64-unknown-hermit image> cargo test -p aspen-jobs --test hermit_uhyve_product_path_test --features plugins-vm hermit_uhyve_executes_declared_fixture_through_product_orchestration -- --ignored --nocapture`. Prefer `.#uhyve` and `.#hermit-uhyve-marker` outputs for those paths.
- The successful job result contains `ASPEN_HERMIT_UHYVE_RUNTIME_HOST_EXECUTED`, `aspen:runtime-host/hermit-uhyve-v1`, and `engine: uhyve`.
- The successful job records at least one `WorkerPool` execution attempt after submitting a blob-backed `hermit_uhyve` payload.
- The negative guardrail contains `ASPEN_HERMIT_UHYVE_RUNTIME_HOST_PRODUCT_PATH_GUARD` and proves invalid payloads, non-zero exits, and successful Uhyve exits without the marker are not execution evidence.

Do not treat any of the following as sufficient by themselves:

- the OpenSpec archive existing;
- the test-harness inventory listing the row;
- Cloud Hypervisor snapshot files existing;
- an Aspen node starting successfully;
- a package build of `aspen-node-vm-test` without the gated VM check;
- runtime-core-only WASM admission checks;
- plugin install/reload commands without a blob-backed job execution receipt;
- Hyperlight worker construction or job-payload serialization without a `WorkerPool` attempt;
- Hyperlight guest package builds without a blob-backed `vm_execute` job receipt;
- ignored/manual Hyperlight examples or direct worker-only calls without product orchestration;
- OCI admission/model checks without a product-path target host execution receipt;
- OCI lowering/build-only checks without submitting the derived isolated target artifact through Aspen orchestration;
- raw container execution, dev/unsafe container paths, or mutable OCI tags alone;
- fake-Uhyve tests, direct Uhyve shell commands, Hermit image package builds such as `.#hermit-uhyve-marker`, metadata contract checks such as `hermit-uhyve-marker-contract`, skipped/ignored Hermit tests not run on a capable host, or successful Uhyve exits that do not emit `ASPEN_HERMIT_UHYVE_RUNTIME_HOST_EXECUTED`.

## How to reproduce

Run the static acceptance-bundle consistency check before making or refreshing a runtime-host readiness claim:

```bash
scripts/test-harness.sh runtime-host-acceptance-bundle
```

This check verifies docs, suite manifests, generated inventory, proof markers, and non-proof wording. It does not execute gated microVM, Hyperlight, or Hermit/Uhyve proofs.

The WASM and OCI lowering rows are cheap and do not require nested KVM:

```bash
cargo test -p aspen-jobs --test wasm_product_path_test --features plugins-wasm -- --nocapture
cargo test -p aspen-jobs --test oci_lowering_product_path_test --features plugins-wasm -- --nocapture
```

The Hermit/Uhyve row is gated because it requires real Uhyve, a valid Hermit image, and a Uhyve/KVM-capable host:

```bash
uh=$(nix build .#uhyve --no-link --print-out-paths)
marker=$(nix build .#hermit-uhyve-marker --no-link --print-out-paths)
nix build .#checks.x86_64-linux.hermit-uhyve-marker-contract --no-link -L
ASPEN_UHYVE="$uh/bin/uhyve" \
ASPEN_HERMIT_UHYVE_IMAGE="$marker/bin/aspen-hermit-uhyve-marker" \
cargo test -p aspen-jobs --test hermit_uhyve_product_path_test --features plugins-vm hermit_uhyve_executes_declared_fixture_through_product_orchestration -- --ignored --nocapture
```

The Hyperlight row is gated because it requires cargo-hyperlight plus a Hyperlight/KVM-capable host:

```bash
cargo test -p aspen-jobs --test hyperlight_product_path_test --features plugins-vm invalid_hyperlight -- --nocapture
cargo test -p aspen-jobs --test hyperlight_product_path_test --features plugins-vm hyperlight_job_executes_declared_fixture_through_product_orchestration -- --ignored --nocapture
```

The positive Hyperlight test builds `examples/vm-jobs/echo-worker` with `cargo hyperlight build --release` when `ASPEN_HYPERLIGHT_GUEST_BINARY` is not set and the default fixture binary is missing.

The microVM check requires nested KVM and is intentionally not part of cheap/default verification. Run the package gate first so packaging failures do not consume a full VM-test cycle:

```bash
nix build --impure .#packages.x86_64-linux.aspen-node-vm-test --no-link --print-out-paths -L
nix build --impure .#checks.x86_64-linux.vm-snapshot-e2e-test --no-link -L --option sandbox false
```

If only refreshing the derivation path or checking Nix evaluation:

```bash
nix eval --impure .#checks.x86_64-linux.vm-snapshot-e2e-test.drvPath --raw
```

## What the check covers

The accepted `runtime-host-microvm-ci-vm` path currently covers:

1. host-side Aspen cluster/node startup with worker and CI support enabled;
2. VM executor access to the configured Cloud Hypervisor, VirtioFS, kernel, initrd, toplevel, and `ip` binary;
3. host-created TAP attachment to the Aspen CI bridge;
4. direct-boot guest network bootstrap on `eth0` with route to the host bridge;
5. golden snapshot creation with Cloud Hypervisor `memory-ranges` artifacts and Aspen `ticket.txt`;
6. snapshot-restored guest worker registration with the cluster;
7. `ci_vm` job submission using the local-executor payload schema (`command`, `args`, `timeout_secs`);
8. job completion through the restored guest worker;
9. a second job for snapshot-restore latency evidence;
10. an eight-job stress slice that forces concurrent restores and records COW efficiency.

## Boundaries and caveats

- This is a gated, impure nested-KVM acceptance check. It is expected to be run deliberately, not as a default local smoke test.
- The current proven host classes are Cloud Hypervisor microVM CI, product-path WASM jobs, gated product-path Hyperlight jobs, OCI-lowered WASM jobs, and gated product-path Hermit/Uhyve jobs. Remaining metadata-only matrix rows for other host classes are visibility gaps until promoted to runnable checks with receipts.
- The verified E2E path uses `microvm.postBootCommands` to emit readiness markers, configure guest networking, and launch the worker in the direct-boot image. Do not promote direct systemd target boot to a product guarantee without a separate design/spec and fresh E2E evidence.
- The check uses `/tmp` VM state inside the NixOS test guest and removes it during cleanup. Preserve separate `.agent/evidence/...` logs when citing historical evidence.
- Logs and incident notes must redact cluster tickets, `aspen://...` remotes, bearer values, private keys, connection strings, and private checkout/source URLs as `[REDACTED]`.

## Operator checklist

Before citing runtime-host readiness:

- Confirm the commit under review is at or after the implementation commit you intend to cite.
- Run or cite the package gate and gated nested-KVM check command.
- Confirm the log has the proof markers above, not just inventory registration or snapshot artifacts.
- Record the derivation path and evidence log path.
- State the host class precisely: `runtime-host-microvm-ci-vm` / Cloud Hypervisor microVM CI, `runtime-host-wasm-product-path` / product-path WASM job runtime, `runtime-host-hyperlight-product-path` / gated product-path Hyperlight job runtime, `runtime-host-oci-lowering-product-path` / OCI-lowered WASM job runtime, or `runtime-host-hermit-uhyve-product-path` / gated product-path Hermit/Uhyve job runtime.
- State unsupported or metadata-only host classes separately.
- Redact secrets before copying any log excerpts.
