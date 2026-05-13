# VM and microVM proof tier evidence

Captured: 2026-05-13T01:54:26Z

Raw logs are under ignored `target/runtime-proof/`; committed evidence keeps only command/result summaries and proof-boundary classifications. No cluster tickets or private git URLs are retained here.

## Build/input drift repair

During this tier, the nearby VM/microVM checks initially failed before reaching product assertions. The failures classified as Nix/build-input drift rather than clustering behavior:

- generated full/crane source paths could still expose private UCAN git dependency metadata instead of a local materialized flake input;
- the mesh/VirtioFS rails pulled `full-aspen-node-plugins`, whose stale plugin fixture build (`aspen-wasm-plugin`) failed against the current `aspen_core` public surface even though those rails do not exercise plugin loading;
- nested plugin vendor patching needed recursive glob coverage for vendored crates used by full package closures.

Focused `flake.nix` changes materialize the locked UCAN input for VM proof builds, rewrite UCAN workspace dependency tables to local `.nix-inputs/ucan` paths, recursively apply vendor compatibility patches, and route the mesh/VirtioFS checks through the non-plugin full node package so the check surface matches the product path under test.

Validation before the final proof runs:

```bash
git diff --check
nix eval --impure .#checks.x86_64-linux.microvm-virtiofs-net-test.drvPath >/dev/null
```

Result: exit 0.

## Product checks

### `microvm-virtiofs-net-test`

Command:

```bash
set -o pipefail; nix build --impure .#checks.x86_64-linux.microvm-virtiofs-net-test --no-link -L \
  2>&1 | tee target/runtime-proof/microvm-virtiofs-net-after-nonplugin-node.log
```

Result: exit 0.

Proof marker:

```text
=== ALL PHASES PASSED ===
Phase 1: 3-node Raft cluster bootstrapped
Phase 2: VirtioFS daemon connected, data seeded via Raft consensus
Phase 3: aspen-net daemon with SOCKS5 proxy running
Phase 4: Guest A booted with VirtioFS mount + nginx serving from Raft KV
Phase 5: Service published to mesh via CLI
Phase 6: Traffic routed through mesh — content from Raft KV -> VirtioFS -> nginx -> SOCKS5 -> iroh -> curl
test script finished in 45.84s
```

Classification: reached build closure, VM boot, service readiness, and product assertions.

### `microvm-net-mesh-test`

Command:

```bash
set -o pipefail; nix build --impure .#checks.x86_64-linux.microvm-net-mesh-test --no-link -L \
  2>&1 | tee target/runtime-proof/microvm-net-mesh-after-nonplugin-node.log
```

Result: exit 0.

Proof marker:

```text
=== ALL PHASES PASSED ===
Phase 1: 3-node Raft cluster bootstrapped
Phase 2: aspen-net daemon with SOCKS5 proxy running
Phase 3: Guest A HTTP server in Cloud Hypervisor microVM
Phase 4: Service published to mesh via CLI
Phase 5: Guest B client in Cloud Hypervisor microVM
Phase 6: Traffic routed through service mesh (SOCKS5 → iroh → guest A)
test script finished in 43.09s
```

Classification: reached build closure, VM boot, service readiness, and product assertions.

### `microvm-raft-virtiofs-test`

Command:

```bash
set -o pipefail; nix build --impure .#checks.x86_64-linux.microvm-raft-virtiofs-test --no-link -L \
  2>&1 | tee target/runtime-proof/microvm-raft-virtiofs-phase2.log
```

Result: exit 0.

Proof marker:

```text
=== ALL PHASES PASSED ===
Phase 1: 3-node Raft cluster bootstrapped and running
Phase 2: VirtioFS daemon connected, data seeded via Raft consensus
Phase 3: CH microVM booted with VirtioFS mount, nginx responding
Phase 4: End-to-end data path verified (Raft → VirtioFS → nginx → curl)
test script finished in 34.87s
```

Classification: reached build closure, VM boot, service readiness, and product assertions.

### `vm-snapshot-virtiofs-test`

Command:

```bash
set -o pipefail; nix build --impure .#checks.x86_64-linux.vm-snapshot-virtiofs-test --no-link -L --option sandbox false \
  2>&1 | tee target/runtime-proof/vm-snapshot-virtiofs-phase2.log
```

Result: exit 0.

Proof markers:

```text
CH snapshot/restore prerequisites: finished
VirtioFS daemons ready
Boot microVM: finished
Read from nix store VirtioFS: finished
Read/write workspace VirtioFS: finished
Snapshot created
Snapshot files: config.json, memory-ranges, state.json
test script finished in 174.29s
```

Classification: reached build closure, Cloud Hypervisor VM boot, VirtioFS readiness, snapshot creation, restore path, cleanup, and final test-driver success. The cleanup phase logged expected best-effort service-stop noise for already-stopped/unloaded restored services; the Nix check exited 0.

## Boundary

This evidence proves the Phase 2 VM/microVM tier named by the OpenSpec. It does not claim the later runtime-host execution tier, Hermit/uHyve ignored execution proof, Hyperlight ignored execution proof, dogfood/self-hosting, or full `nix flake check`.
