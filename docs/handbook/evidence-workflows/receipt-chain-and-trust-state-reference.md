# Receipt, chain, and trust-state reference

Mode: Reference

Use this page while labeling refs in an evidence inventory. It distinguishes storage identity, scoped continuity, and provenance evaluation rather than treating every receipt as one interchangeable proof type. It complements the [Handbook](../README.md) and [evidence/authority technical article](../../technical/foundations/evidence-and-authority-separation.md). The [proof workflow](../../proof-workflow.md) governs claim scope and review obligations; this reference reports inspected implementation behavior. No commands or tests were executed for this batch.

## Object identity and ownership

| Object or result | Important fields | Owner and interpretation |
| --- | --- | --- |
| Ledger `Entry` | `artifact_ref`, `artifact_kind` | Ledger inventory; kind classification is not full semantic admission. |
| Ledger `Import` | `artifact_ref`, `artifact_kind`, `receipt_value` | Import stores the requested value and returns an import receipt value. It does not recursively store that returned receipt. |
| Ledger `Export` | `artifact_ref`, `artifact_kind`, `receipt_value` | Export reads and rehashes stored content, writes Preserves text, and returns export evidence. |
| `ChainScope` | `scope`, `id`, `epoch` | Evidence-chain identity domain; equal sequence numbers in different tuples do not imply a shared order. |
| `ChainPayload` | `kind`, `artifact_ref`, `schema` | Descriptor for the unchanged payload named by a link. |
| `ChainLinkInput` | `chain`, `sequence`, `previous_link_ref`, `payload`, `context_refs`, `producer`, `trellis`, `checks` | Pure link construction inputs; serialization alone is not ledger append admission. |
| `ChainAppend` | `link_ref`, `payload_ref`, `head_before`, `head_after`, `predicate_receipt_ref`, `receipt_ref`, `receipt_value` | Append result; this path explicitly stores the link and append receipt. |
| Provenance `Evaluation` | `decision`, `receipt_ref`, `receipt_value`, `matched_record_ref`, `diagnostics` | Subject/operation/profile-specific provenance decision, not execution authority. |

Canonical Preserves bytes and BLAKE3 define content identity. Ledger files use the `content/blake3_<hex>.bin` naming convention internally. `read_artifact` parses canonical bytes and checks their hash against the requested ref. A pathname, Rust memory layout, or rendered summary is not a substitute identity.

## Provenance records and build evidence

The record parser accepts the current `provenance-record-v1` form with `build-records` and an older form without that field. The latter yields an empty build-record list; syntactic acceptance must not be mistaken for a satisfied reproducible-build binding.

| Record | Wire field groups | What the consumer must keep distinct |
| --- | --- | --- |
| `provenance-record-v1` | `artifact`, `trust-state`, `source`, `dependency-closure`, `toolchain`, `builder`, `review`, `tests`, `source-gates`, `policy`, `build-records` | The record's own canonical ref differs from its artifact subject. Referenced review/policy objects are not replaced by naming them. |
| `provenance-build-record-v1` | `expected-artifact`, `source`, `dependency-closure`, `toolchain`, `build-params`, `builder`, `nix-derivations`, `policy`, `evidence` | Describes expected inputs/output; constructing it does not execute a build. |
| `provenance-build-verify-receipt-v1` | `decision`, `expected-artifact`, `actual-artifact`, `build-record`, `diagnostics` plus schema/boundary fields | Records comparison with a supplied actual ref; a matching decision is not independent measurement of an executable. |

The public types and parsing code own these fields. Provenance ref collections and evaluation input collections are bounded at 64. Build parameters are bounded at 64, and each key/value token is nonempty, at most 256 bytes, and contains no newline or carriage return. Bounds constrain input handling; they are not completeness guarantees for a supply-chain review.

## Trust-state and threshold lookup

| State | Ordinary `local-test` | Ordinary `node-control` | Explicit sensitive operation |
| --- | --- | --- | --- |
| `unknown`, `source-known`, `builder-attested`, `denied` | Not admitted | Not admitted | Not admitted |
| `sandbox-only` | Admitted by threshold | Not admitted | Not admitted |
| `reviewed` | Admitted by threshold | Admitted by threshold | Not admitted |
| `reproducible-verified` | Threshold plus build binding | Threshold plus build binding | Threshold plus build binding |
| `policy-trusted` | Admitted by threshold | Admitted by threshold | Admitted by threshold |

“Admitted by threshold” is not an overall pass: malformed inputs and prior diagnostics can still deny evaluation. Sensitive operations are the exact strings `install-policy-artifact`, `install-migration-recipe`, `install-production-executable`, and `remote-sync-execute`. The build-binding branch is specific to reproducible-verified records. In particular, the threshold's `reproducible_build_verification_required` metadata does not demonstrate that policy-trusted records take that branch.

## Inspection surface lookup

These are source spellings, not executable recipes. Nesting is declared in the [root command parts](../../../src/main/root/parts/command/p000/body.rs), with module aliases in [main](../../../src/main.rs).

| Surface | Use | Source owner |
| --- | --- | --- |
| `test ledger list` / `export` | Inventory all classified artifacts; materialize a ref as Preserves text | [Ledger declaration and arguments](../../../src/cli/ops/ledger/command.rs) |
| `test provenance show` | Render provenance-specific artifacts | [Provenance declaration](../../../src/cli/workflow/provenance/command.rs) |
| `test provenance evaluate` | Produce evaluation evidence from explicit subject/profile inputs | [Provenance handler](../../../src/cli/workflow/provenance/ops.rs) |
| `receipts list` / `show` / `validate` | Inspect supported dogfood/operator receipt kinds, not every ledger object | [Operator receipt implementation](../../../src/cli/evidence/receipts/operator.rs) |
| `test chain publish` / `fetch` | Exchange chain segments; not an inspection-only append/verify interface | [Chain declaration](../../../src/cli/ops/ledger/command.rs) |

## Worked classification failure

Suppose a handoff labels a `chain-append-receipt-v1` ref as “the artifact.” Follow the payload ref to locate the actual subject; follow the link ref for continuity; retain the append receipt as evidence of that append path. A passing provenance receipt for the payload cannot replace chain continuity evidence, and a valid chain cannot replace provenance evaluation. The same discipline prevents exporting a signed wrapper when the recipient actually requested its underlying subject.

Chain segment verification defaults to rejecting unexpected forks; the diagnostic retain policy does not choose a globally authoritative branch. Ledger scans and evidence-chain links have bounds of 100,000 in the inspected modules. Segment verification stores predicate/verification evidence, so even a function named “verify” is not necessarily filesystem-read-only.

## Sources

- [Handbook](../README.md)
- [Evidence and authority separation](../../technical/foundations/evidence-and-authority-separation.md)
- [Proof workflow](../../proof-workflow.md)
- [Ledger public operations](../../../src/ledger/parts/mod/p000/body.rs)
- [Chain types](../../../src/evidence/parts/chain/p000/body.rs)
- [Chain verification effects](../../../src/evidence/parts/chain/p003/body.rs)
- [Provenance types](../../../src/provenance/parts/mod/p000/body.rs)
- [Provenance binding and thresholds](../../../src/provenance/parts/mod/p002/body.rs)
- [Build parameter validation](../../../src/provenance/parts/mod/p005/body.rs)
