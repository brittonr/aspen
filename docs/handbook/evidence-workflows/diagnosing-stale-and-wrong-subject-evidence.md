# Diagnosing stale and wrong-subject evidence

Mode: Troubleshooting

Use this guide when evidence exists but does not support the requested operation. “Stale” is not one universal timestamp check: a receipt can name an earlier artifact, a chain anchor can belong to another epoch, or a build verification can reference a build record no longer selected by the provenance record. The [proof workflow](../../proof-workflow.md) requires explicit subject and layer bindings; the [technical companion](../../technical/foundations/evidence-and-authority-separation.md) explains why availability is not authority. Start from the [Handbook](../README.md) when the failure belongs to another subsystem.

This guide is source-checked, not runtime-verified. The cited negative tests are checked-in examples, not reproduced failures from this documentation batch. Preserve original bytes, requested refs, operation/profile, and diagnostics before taking action.

## Symptom: the requested ref cannot be read

**Discriminating evidence:** distinguish malformed identity, absent content, and corrupt materialized bytes. The ledger negative tests separately reject shorthand refs such as `blake3:fixture`, a valid-shaped but absent ref, and altered canonical content stored under an original ref. A filename that looks content-addressed does not prove its contents match.

**Safe next action:** confirm the complete canonical ref from the producer's receipt and identify the intended ledger root. Inventory that root without importing or deleting anything. The guarded command below is supported by the [root declaration](../../../src/main/root/parts/command/p000/body.rs), [ledger CLI declaration](../../../src/cli/ops/ledger/command.rs), and [list handler](../../../src/cli/ops/ledger/ops.rs), connected by [main aliases](../../../src/main.rs). Source-checked; not executed.

```sh
: "${LEDGER_ROOT:?Set the existing evidence ledger root}"
test -d "$LEDGER_ROOT/content" &&
  molten test ledger list --ledger "$LEDGER_ROOT"
```

Listing reads and rehashes recognized content files; corruption can stop the inventory. An empty list alone is not proof that a producer never emitted evidence. **Stop** on a content-hash mismatch: preserve the damaged store and obtain a separately verified copy rather than replacing bytes in place or deleting state.

## Symptom: provenance says no record matches the artifact

**Discriminating evidence:** compare the requested subject with each record's `artifact`, not the record's own ref. The evaluator emits missing-evidence diagnostics for an empty input collection and a no-matching-record diagnostic when supplied records name other artifacts.

**Worked failure:** `reviewed_provenance_passes_node_control_and_wrong_artifact_denies` first evaluates a synthetic reviewed record against its subject, then supplies a different canonical artifact ref. The latter denies without making the original record malformed. This is the difference between identity correctness and claim applicability.

**Safe next action:** obtain the provenance for the intended artifact, or correct the request only when independent evidence establishes a selection mistake. Retain the old evaluation. **Stop** if the requested subject cannot be established; editing the record to make its `artifact` agree would create a different claim, not repair the evidence.

## Symptom: a reproducible record still denies

**Discriminating evidence:** inspect the receipt's decision, both artifact refs, and build-record binding. The binding helper requires a passing candidate with expected and actual refs equal to the requested artifact and a build-record ref included by the selected provenance record. A passing verification for yesterday's build record does not satisfy a newly selected record merely because its summary looks similar.

**Safe next action:** collect the exact missing build record and verification receipt, check their canonical refs, and evaluate into a new output file. `verify-build` compares caller-supplied refs; it is not a rebuild command. **Stop** if there is no independently established actual artifact ref or if the producer cannot supply the bound build evidence. Do not generate a matching verification from an assumed actual ref.

## Symptom: local fixture evidence passes but node-control denies

**Discriminating evidence:** inspect `profile`, operation spelling, and `trust-state`. Sandbox-only satisfies ordinary local-test evaluation but not ordinary node-control. Sensitive operation names require the stronger threshold. The parser accepts several non-admitted states because records can describe unknown or denied trust.

**Safe next action:** request evidence appropriate to the actual consumer operation. **Stop** rather than downgrading the profile, renaming the operation, or manufacturing a reviewed fixture to replace missing evidence. The synthetic fixture helper is for deterministic examples, not release authority.

## Symptom: chain verification disagrees with a supplied anchor

**Discriminating evidence:** record the entire `(scope, id, epoch)` tuple. The verifier distinguishes `missing-anchor` from `anchor-chain-mismatch`; append validation separately checks previous ref, sequence increment, and scope. A valid anchor from another epoch cannot establish this segment's continuity.

**Safe next action:** resolve the requested anchor/head and their payloads from preserved evidence, then have the owning workflow verify the intended segment. Be aware that the library verifier stores predicate and verification evidence; it is not a read-only filesystem diagnostic. **Stop** on ambiguous forks or missing scope context. Diagnostic fork retention is not permission to select a production branch, and deletion is not continuity repair.

## Symptom: the command succeeded, but the receipt denies

**Discriminating evidence:** the provenance CLI writes a normal deny evaluation and returns `Ok(())`; shell success means the handler completed, not that provenance passed. Conversely, `receipts show` can reject a provenance receipt because the operator reader supports different kinds, not because the stored bytes are corrupt.

**Safe next action:** use the provenance-specific reader and inspect the canonical receipt's decision and diagnostics. Its short summary does not show every binding. **Stop** any downstream mutation until the owning gate admits the operation. A fresh receipt, a long chain, or a successful export cannot replace that gate.

## Sources

- [Handbook](../README.md)
- [Proof workflow](../../proof-workflow.md)
- [Evidence and authority separation](../../technical/foundations/evidence-and-authority-separation.md)
- [Ledger negative tests](../../../src/ledger/parts/mod/tests/m000/p000/body.rs)
- [Wrong-subject provenance fixture](../../../src/provenance/parts/mod/tests/m000/p000/body.rs)
- [Provenance matching](../../../src/provenance/parts/mod/p001/body.rs)
- [Build binding and thresholds](../../../src/provenance/parts/mod/p002/body.rs)
- [Chain anchor diagnostics](../../../src/evidence/parts/chain/p003/body.rs)
- [Provenance shell decisions](../../../src/cli/workflow/provenance/ops.rs)
