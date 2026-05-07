# Final validation evidence

Validated the OCI artifact-lowering change after all tasks were marked complete.

## Commands

```bash
rustfmt crates/aspen-runtime-core/src/lib.rs --check
CARGO_TARGET_DIR=target/agent cargo test -p aspen-runtime-core --all-targets
CARGO_TARGET_DIR=target/agent cargo test --test runtime_host_loading_docs_test runtime_applications_doc_anchors_host_loading_taxonomy
python - <<'PY'
from pathlib import Path
doc = Path('docs/runtime-applications.md').read_text()
required = [
    'OciLoweringPlan',
    'OciLoweringTarget',
    'content-addressed packaging/compatibility artifact',
    'MicroVm` by default',
    'Hyperlight`, `Wasm`, or a VM-backed `Unikernel`',
    'Plain Podman/Docker-style host containers are dev/unsafe-only',
    'rejected as the default production boundary',
]
missing = [needle for needle in required if needle not in doc]
if missing:
    raise SystemExit(f'missing OCI docs anchors: {missing}')
print('oci runtime docs anchors present')
PY
openspec validate lower-oci-artifacts-to-isolated-hosts --strict
git diff --check
```

## Results

- `cargo test -p aspen-runtime-core --all-targets`: 33 passed.
- `runtime_host_loading_docs_test`: 1 passed.
- Docs anchor assertion: `oci runtime docs anchors present`.
- OpenSpec strict validation: `Change 'lower-oci-artifacts-to-isolated-hosts' is valid`.
- Whitespace check passed.

No secrets, credentials, bearer tokens, registry credentials, raw environment secrets, mutable tags as durable identity, ambient host paths, or private material were emitted in evidence.
