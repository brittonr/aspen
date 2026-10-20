# Full repository flake gate — 2026-10-19

## Command

```sh
nix flake check -L --no-update-lock-file
```

## Result

- Exit code: `0`
- Output: `all checks passed!`
- Wall time: 916.54 seconds
- Source checkout: `a0b762ff88934b807df56ce145e13881f808ad61`
  (working tree otherwise clean except the operator-owned `AGENTS.md`)
- `molten-nextest`: 1,746 tests run, 1,746 passed, 0 skipped
- CI receipt:
  `blake3:f62ce024b4be5cca4397875b9c06f8d1664bef3b7ffe9cd32e0e6ee7a29bca79`
  with decision `pass`
- `molten-dogfood-local-node` release verification, bundle verification,
  promotion, and export verification all passed.
- The check omitted the incompatible systems `aarch64-darwin`, `aarch64-linux`,
  and `x86_64-darwin`; this is the default x86_64-linux flake gate.

The preceding full-gate attempt failed on inherited Tracey debt inherited from
the archived preflight batch. That debt was recorded in
`evidence/tracey/2026-10-19-inherited-preflight-debt.md` before this passing
run. The local daemon lock recorded in the attempt-21 evidence did not block
this run.

## Non-claims

This passes the repository flake gate for the recorded x86_64-linux source and
check cohort. It does not establish whole-system correctness, production
readiness, release eligibility, or remote-push authorization.
