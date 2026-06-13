## Context

Octet now loads from `[workspace.metadata.octet]` and writes deterministic run artifacts under `target/octet`:

- `command.txt`
- `status.json`
- `summary.txt`
- optional structured results such as JSONL/SARIF
- Valence/object corpus receipts such as `target/octet/object-corpus-receipt.json`

The first runs established useful evidence but showed that process success is not a pass condition: `warning-only` means the tool ran and produced evidence; it does not mean the source passed Molten's evidence policy. Current strict runs are configuration-clean with disabled lint families documented as caveats. A fail-closed gate must not confuse `cargo octet` process success, raw summaries, or quarantine evidence with strict source admissibility.

## Gate policy

Introduce a canonical gate policy record:

```preserves
<octet-gate-policy-v1
  "molten.octet.gate-policy.v1"
  <profile "strict-ci">
  <command ["cargo" "octet" "check" "--artifact-dir" "target/octet"]>
  <required-artifacts ["command.txt" "status.json" "summary.txt" "object-corpus-receipt.json"]>
  <deny-statuses ["error" "warning-only" "missing" "malformed" "stale"]>
  <critical-lints ["no_panic" "no_unwrap" "ambient_clock" "unbounded_loop" "secret_rendering" "harness_backdoor" "authority_typing"]>
  <quarantine-policy <none>>
  <checks [...]>>
```

A quarantine-enabled profile may name an explicit quarantine/baseline receipt, but strict release/admission/upgrade profiles should converge on no quarantine allowance.

## Gate receipt

The gate receipt should be canonical Preserves, hashable, ledger-classified, and suitable for use by harness, release, upgrade, and node-runtime startup evidence:

```preserves
<octet-gate-receipt-v1
  "molten.octet.gate-receipt.v1"
  <decision "pass"|"deny">
  <policy-ref "b3:...">
  <command-ref "b3:...">
  <status-ref "b3:...">
  <summary-ref "b3:...">
  <findings-ref <some "b3:...">|<none>>
  <object-corpus-ref <some "b3:...">|<none>>
  <fingerprint-ref <some "b3:...">|<none>>
  <baseline-ref <some "b3:...">|<none>>
  <review-refs [...]>
  <counts <findings n> <warnings n> <errors n> <critical n> <uncovered n>>
  <diagnostics [...]>
  <checks [...]>>
```

Pass receipts require:

1. the Octet command completed and wrote all required artifacts;
2. artifact refs match the configured command/profile/toolchain;
3. the status is strict-pass, or every finding is covered by an allowed and unexpired quarantine profile;
4. critical lints are absent or explicitly covered by a review receipt allowed for the exact surface/profile;
5. object corpus and fingerprint evidence are present for configured critical paths;
6. the receipt itself is stored in the local evidence ledger or exported as a content-ref artifact.

## Fail-closed rules

The gate denies on:

- missing `status.json`, `summary.txt`, `command.txt`, object corpus receipt, or structured finding artifact required by the profile;
- stale config/profile hash relative to `[workspace.metadata.octet]`;
- unsupported cargo-octet version or toolchain without review receipt;
- `warning-only` in strict profile;
- any new finding not covered by a quarantine/baseline receipt in quarantine profile;
- any critical finding without a matching human/policy review receipt;
- findings whose file/line/message cannot be parsed into a stable finding key;
- artifacts that mention only process success but not source-gate decision.

## CI shape

Initial strict command sequence:

```text
cargo octet check --artifact-dir target/octet
cargo octet check -p molten --artifact-dir target/octet-lib -- --lib
cargo octet object corpus receipt --output target/octet/object-corpus-receipt.json <critical paths>
molten test octet artifacts import --artifacts target/octet --ledger target/octet-ledger --receipt-out target/octet/artifact-ledger-receipt.preserves
molten test octet gate --artifacts target/octet --profile strict-ci --receipt-out target/octet/gate-receipt.preserves
molten test octet remediation plan --artifacts target/octet --lib-artifacts target/octet-lib --focused-object-corpus target/octet/object-corpus-receipt.json --receipt-out target/octet/remediation-plan.preserves
cairn validate --strict
```

During burn-down, CI may use a separate `quarantine-ci` profile to block new warnings while preserving the current warning snapshot. Release/admission/upgrade/node startup profiles require strict validation receipts before claiming source-gate pass evidence.

## Non-goals

- Do not claim Octet proves semantic correctness.
- Do not hide warning findings by deleting artifacts.
- Do not add permanent allow-all suppressions.
- Do not let process exit code `0` stand in for a gate pass receipt.
