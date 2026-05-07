# MicroVM final validation

- Change: `implement-microvm-runtime-runner`
- Task: focused tests, strict OpenSpec validation, helper verification, whitespace checks
- Started: `2026-05-07T02:57:44Z`
- Completed: `2026-05-07T02:58:09Z`

## Commands

```bash
rustfmt crates/aspen-runtime-core/src/lib.rs --check
CARGO_TARGET_DIR=target/agent cargo test -p aspen-runtime-core --all-targets
python - <<'PY'
from pathlib import Path
text=Path('docs/runtime-applications.md').read_text()
required=['MicroVmRuntimeProfile','MicroVmEngine','Firecracker, Cloud Hypervisor, Uhyve, QEMU microvm','virtualization backend','runner capability/version','supported guest artifact profile','LinuxGuest','Unikernel','declared launch bindings','lease/heartbeat','RuntimeHostKind::MicroVm']
missing=[x for x in required if x not in text]
if missing:
    raise SystemExit(f'missing docs anchors: {missing}')
print('microvm runtime docs anchors present')
PY
openspec validate implement-microvm-runtime-runner --strict
git diff --check
python ~/.hermes/skills/agentkit-port/openspec/scripts/openspec_helper.py verify implement-microvm-runtime-runner --json
```

## Result

- Runtime-core unit tests: `26 passed; 0 failed`.
- Docs source-anchor assertion printed `microvm runtime docs anchors present`.
- Strict OpenSpec validation passed.
- Whitespace check passed.
- Initial helper verification reported only the intentionally in-progress final task; this evidence closes that task before the final pre-archive helper run.
