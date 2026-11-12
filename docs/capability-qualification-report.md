# Capability qualification report

`tools/capability-report.rs` is a read-only report that joins the reviewed
[evidence matrix](../tests/evidence-matrix.ncl) with an operator-supplied bundle
of exact execution receipts. It keeps implementation coverage and recorded
execution separate.

This is diagnostic input to a release review, not a release decision. A
`bounded-observed` row names a finite scenario and its receipts. The report does
not grant authority, select a production profile, or reinterpret an existing
receipt.

## Inputs

Export the matrix once per reviewed revision:

```sh
nickel export --format json tests/evidence-matrix.ncl > /tmp/evidence-matrix.json
```

The report input names one exact build, closure, feature set, profile, platform,
and the capabilities to qualify:

```json
{
  "schema": "molten.capability-report.v1",
  "source_revision": "dae9d492e",
  "dependency_closure_ref": "blake3:...",
  "feature_set": ["default"],
  "execution_profile": "nixos-vm",
  "platform": "x86_64-linux",
  "capabilities": [
    {
      "id": "durable-state-recovery",
      "requirements": ["molten.fabric_consistency.live_raft"],
      "profiles": ["nixos-vm"],
      "evidence": [
        {
          "requirement_id": "molten.fabric_consistency.live_raft",
          "receipt_path": "target/run/recovery.json",
          "declared_ref": "blake3:...",
          "subject_ref": "blake3:...",
          "scope_ref": "blake3:...",
          "decision": "pass"
        }
      ]
    }
  ]
}
```

Each receipt file is JSON with the bounded schema:

```json
{
  "schema": "molten.capability-evidence.v1",
  "requirement_id": "molten.fabric_consistency.live_raft",
  "subject_ref": "blake3:...",
  "scope_ref": "blake3:...",
  "decision": "pass",
  "caveats": ["finite three-voter profile only"]
}
```

A receipt is accepted only when its file bytes hash to `declared_ref`, its
schema is exact, and its requirement, subject, scope, and decision match the
report entry. A missing file, placeholder hash, wrong subject, wrong scope, or
unexecuted ignored test cannot become `bounded-observed`.

## Statuses

| Status | Meaning |
| --- | --- |
| `bounded-observed` | At least one validated receipt with `pass` for the capability. |
| `failed` | A validated receipt with `fail`; the report keeps the evidence ref. |
| `inconclusive` | Evidence is present but unvalidated, mismatched, or explicitly inconclusive. |
| `not-recorded` | The matrix has entries but no validated receipt was supplied. |
| `unsupported` | No matrix entry and no exemption, or only a reviewed exemption. |

The report retains the matrix coverage kinds, scopes, targets, and the exact
evidence refs. It does not treat a matrix entry, command string, JUnit file, or
stdout log as execution.

## Run

```sh
cargo -q -Zscript tools/capability-report.rs \
  --matrix /tmp/evidence-matrix.json \
  --input capability-input.json \
  --markdown-out capability-report.md \
  --json-out capability-report.json
```

The tool also exposes `--self-test`, which covers a valid receipt, a placeholder
ref, and a missing receipt.
