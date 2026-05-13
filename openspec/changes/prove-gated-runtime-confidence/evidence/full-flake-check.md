# Full flake check evidence

Captured: 2026-05-13T05:35:26Z

Raw log is kept under ignored `target/runtime-proof/nix-flake-check-max-jobs-1.log`. The raw log contains verbose VM debug output, including trust-share byte arrays, so committed evidence includes only a redacted summary.

## Command

```bash
mkdir -p target/runtime-proof && set -o pipefail; \
  nix flake check -L --max-jobs 1 \
  2>&1 | tee target/runtime-proof/nix-flake-check-max-jobs-1.log
```

Result: exit 0.

## Terminal markers

```text
all checks passed!
warning: The check omitted these incompatible systems: aarch64-darwin, aarch64-linux, x86_64-darwin
Use '--all-systems' to check all.
```

## Boundary classification

The serialized local `nix flake check -L --max-jobs 1` completed successfully on `x86_64-linux`. This proves the repository's configured local flake checks for the current system, including the VM checks reached by the flake check. It does not prove omitted non-current systems.
