# Hermit/uHyve gated product proof evidence

Captured: 2026-05-13T03:09:33Z

Raw logs are kept under ignored `target/runtime-proof/hermit-uhyve-gated-proof.log`; this committed summary omits full build logs and contains no cluster tickets or credentials.

## Command

```bash
set -o pipefail; {
  uhyve=$(nix build .#uhyve --no-link --print-out-paths -L)
  marker=$(nix build .#hermit-uhyve-marker --no-link --print-out-paths -L)
  nix build .#checks.x86_64-linux.hermit-uhyve-marker-contract --no-link -L
  ASPEN_UHYVE="$uhyve/bin/uhyve"   ASPEN_HERMIT_UHYVE_IMAGE="$marker/bin/aspen-hermit-uhyve-marker"     cargo test -p aspen-jobs --test hermit_uhyve_product_path_test --features plugins-vm       hermit_uhyve_executes_declared_fixture_through_product_orchestration -- --ignored --nocapture
} 2>&1 | tee target/runtime-proof/hermit-uhyve-gated-proof.log
```

Result: exit 0.

## Built inputs

- Uhyve runner: `/nix/store/dv1bczsyhh7lc2l8nfcz7396agr6x4pd-uhyve-0.8.0-unstable-2026-05-08/bin/uhyve`
- Hermit marker fixture: `/nix/store/gdriy4giigz47yy5dw0ydk8kw2fy6pyh-hermit-uhyve-marker-0.1.0-unstable-2026-05-08/bin/aspen-hermit-uhyve-marker`
- Marker metadata contract: `.#checks.x86_64-linux.hermit-uhyve-marker-contract`, exit 0.

## Proof markers and boundary

Test output:

```text
test hermit_uhyve_executes_declared_fixture_through_product_orchestration ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.15s
```

The gated test asserts the completed `hermit_uhyve` job:

- passed through `WorkerPool` orchestration (`attempts > 0`);
- returned `JobStatus::Completed` with `JobResult::Success`;
- set the product-visible receipt marker to `ASPEN_HERMIT_UHYVE_RUNTIME_HOST_EXECUTED`;
- included that marker in real Uhyve serial stdout or stderr from the packaged Hermit marker image.

Classification: reached the final Hermit/uHyve runtime-host product receipt boundary on this host: packaged runner and fixture built, fixture metadata contract passed, real Uhyve executed the packaged Hermit image through Aspen `JobManager`/`WorkerPool` orchestration, and the product-visible receipt marker was asserted by the ignored gated test.
