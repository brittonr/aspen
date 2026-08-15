# Design: Operator Dogfood Nix Release Check

## Overview

The release check is a Nix derivation named `molten-dogfood-local-node`. It references the existing `nextest` check output so Nix must build the hermetic test suite before the dogfood release gate. The derivation then runs the built `molten` binary:

```sh
molten dogfood local-node \
  --state-root "$TMPDIR/dogfood-state" \
  --out dogfood-report.preserves \
  --release-gate-out release-gate.preserves
```

The derivation fails unless the command exits successfully, the summary contains `decision=pass`, the report contains `dogfood-report-v1`, and the release gate contains `release-gate-receipt-v1`.

## Evidence outputs

The check copies these files to `$out`:

- `dogfood-summary.txt`
- `dogfood-report.preserves`
- `release-gate.preserves`
- `after-nextest.txt` containing the referenced nextest output path

These outputs are review evidence only. The dogfood release gate remains a canonical release-readiness artifact, not a source of operational authority.

## Failure behavior

The derivation fails closed on missing report/gate files, non-pass dogfood decision, missing release gate receipt, or nextest check failure. It does not mutate repository state and uses only a Nix build temporary state root.
