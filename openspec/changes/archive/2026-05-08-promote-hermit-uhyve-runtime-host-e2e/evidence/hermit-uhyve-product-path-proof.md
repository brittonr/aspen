# Hermit/Uhyve runtime-host product-path proof

Date: 2026-05-08

## Result

Passed. The Hermit/Uhyve runtime-host row now has real Aspen-spawned execution evidence on this host.

## Preconditions

- Real Uhyve binary built from Aspen's flake package:
  - command: `nix build .#uhyve --no-link --print-out-paths`
  - executable: `<nix-store>/bin/uhyve`
- Real Hermit image accepted by Uhyve:
  - source: `hermit-os/uhyve` test-kernels crate with local `src/bin/aspen_marker.rs`
  - target: `x86_64-unknown-hermit`
  - marker printed by guest: `ASPEN_HERMIT_UHYVE_RUNTIME_HOST_EXECUTED`
- Host had sufficient virtualization support for Uhyve to boot the Hermit image.

## Direct Uhyve sanity check

The real Uhyve binary booted the marker image directly before the Aspen product-path proof. The log included:

```text
Hermit is running on uhyve!
ASPEN_HERMIT_UHYVE_RUNTIME_HOST_EXECUTED
```

Direct Uhyve execution is a prerequisite sanity check only; it is not the runtime-host row proof.

## Aspen product-path proof

Command shape:

```bash
uh=$(nix build .#uhyve --no-link --print-out-paths)
ASPEN_UHYVE="$uh/bin/uhyve" \
ASPEN_HERMIT_UHYVE_IMAGE=/tmp/uhyve-src/tests/test-kernels/target/x86_64-unknown-hermit/debug/aspen_marker \
cargo test -p aspen-jobs --test hermit_uhyve_product_path_test --features plugins-vm hermit_uhyve_executes_declared_fixture_through_product_orchestration -- --ignored --nocapture
```

Result:

```text
running 1 test
test hermit_uhyve_executes_declared_fixture_through_product_orchestration ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out
```

The test submits a blob-backed `hermit_uhyve` job through `JobManager`, runs it through `WorkerPool` with `HermitUhyveWorker`, invokes the real Uhyve binary, and requires the product receipt to contain `ASPEN_HERMIT_UHYVE_RUNTIME_HOST_EXECUTED`.

## Guardrails

- Fake-Uhyve tests remain guardrails only and are not runtime-host evidence.
- A direct Uhyve shell run remains a prerequisite sanity check only.
- Package builds alone remain non-proof.
- The worker now fails successful Uhyve exits that do not emit the expected marker, recording `"marker":"missing"` in the bounded receipt.

## Secret-safety

No credentials, cluster tickets, cookies, private keys, registry credentials, or connection strings were used or recorded. Serial output is bounded and secret-like tokens are redacted by the receipt wrapper.
