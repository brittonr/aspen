## Why

Hermit/Uhyve is now a proven gated runtime-host row, but the proof still depends on an operator-supplied `ASPEN_HERMIT_UHYVE_IMAGE` path. The last successful proof used a valid local `/tmp` marker image built from Uhyve test-kernels, which makes reproduction harder and risks future reruns drifting back to invalid ad hoc binaries.

## What Changes

- Add a reproducible Aspen-owned Hermit marker fixture package that builds a valid `x86_64-unknown-hermit` image emitting `ASPEN_HERMIT_UHYVE_RUNTIME_HOST_EXECUTED`.
- Wire the gated Hermit/Uhyve product-path harness/docs to prefer the packaged fixture while preserving explicit opt-in execution and KVM/Uhyve requirements.
- Add cheap package/contract checks proving the fixture exists, records source/provenance, and is not mistaken for execution evidence by itself.

## Out of Scope

- Promoting Hermit/Uhyve to default CI; the row remains gated because real Uhyve and virtualization support are still required.
- Treating package builds, direct Uhyve shell runs, or fixture existence as runtime-host readiness evidence without the Aspen `JobManager`/`WorkerPool` receipt.
- Adding broad Hermit application packaging beyond the single marker fixture needed for proof reproduction.

## Verification

- `nix build .#hermit-uhyve-marker --no-link -L`
- focused package contract check for marker path/provenance
- gated Hermit/Uhyve product-path test using `.#uhyve` and the packaged marker image on a capable host
- `scripts/test-harness.sh export && scripts/test-harness.sh check`
- `openspec validate package-hermit-uhyve-marker-fixture --strict`
- `openspec validate --all --strict --json`
- `git diff --check`
