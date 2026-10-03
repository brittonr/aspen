# Diagnosing catalog identity and access failures

Mode: Troubleshooting

Use this guide to classify a failed or surprising local catalog observation without changing storage or weakening visibility. It is based on inspected source and checked-in fixtures; none of the scenarios was reproduced or executed for this page. Preserve the exact request, supplied root roles, decision, diagnostics, and output references in an appropriately restricted record. Never place confidential payloads in a general support report.

The [inspection how-to](inspecting-catalog-results-without-executing-artifacts.md) contains source-backed command recipes. This page focuses on discriminating evidence and stopping rules rather than asking you to rerun an uncertain operation.

## Symptom: a short identifier is denied

**Discriminating evidence:** distinguish malformed full references, invalid prefix characters, insufficient length, no visible candidates, and multiple visible candidates. [Prefix classification](../../../src/catalog/parts/mod/p008/body.rs) accepts lowercase hexadecimal prefixes; a string carrying the content-reference prefix is validated as a full reference, not silently treated as abbreviated hexadecimal. [Outcome selection](../../../src/catalog/parts/mod/p002/body.rs) defaults to a minimum of eight hex characters through the caller and denies ambiguity rather than choosing a candidate.

**Safe next action:** obtain the full reference from the authorized producer or an already inspected result. If a prefix was truncated during copying, restore it from that evidence. Keep the same visibility context when comparing candidates.

**Stop condition:** do not lower the minimum length to force a choice, hide competing candidates to manufacture uniqueness, or guess a reference from a name. A short ID is a UI convenience over the visible corpus, not durable identity.

## Symptom: a full reference is accepted but viewing fails

**Discriminating evidence:** [reference resolution](../../../src/catalog/parts/mod/p006/body.rs) accepts a well-formed, nonhidden full reference before proving its storage presence. The [view branch](../../../src/catalog/parts/mod/p000/body.rs) then tries the registry and, if supplied, a ledger. A format-valid reference therefore need not be locally available. When a ledger exists, registry-read failure enters the ledger fallback branch; the final error alone need not describe the original registry failure.

**Safe next action:** establish whether the expected object is a registry artifact, payload identity, ledger object, summary, or receipt. Confirm the root roles from the original acquisition record. A receipt's result reference is not the queried artifact's reference.

**Stop condition:** do not import arbitrary bytes, rewrite a name pointer, or delete state to make the reference appear. Content availability and content identity are separate questions.

## Symptom: the result is empty but the decision is pass

**Discriminating evidence:** ordinary completed queries use `pass` even when no items survive. Search filters are ANDed. Hidden top-level objects are skipped. A registry-only query does not search an omitted ledger. Root scope and artifact-kind versus ledger-kind choices can exclude otherwise discoverable evidence.

**Safe next action:** compare the actual request with the intended question. In particular, `search_transcripts` defaults to status `pass` and `list_upgrade_sessions` to `planned` when no filters are supplied. Check the [MCP adapters](../../../src/catalog/parts/mcp/p001/body.rs) before interpreting these names as complete listings.

**Stop condition:** empty discovery is not proof of deletion, revocation, failed execution, or absence from remote stores. Do not remove a visibility restriction just to turn emptiness into a match.

## Symptom: MCP denied a call although the process completed

**Discriminating evidence:** parsing happens before allow-list dispatch. A malformed request can return an error immediately. A parsed but unlisted tool produces a deny response. An allowed tool whose catalog operation errors also produces a deny response, carrying diagnostics. The [CLI handler](../../../src/cli/core/catalog/ops.rs) emits a successfully constructed call and returns success without converting its deny decision into a process error.

**Worked failure case:** the [MCP negative fixture](../../../src/catalog/parts/mcp/tests/m000/p000/body.rs) asks for `catalog.install`. The tool is not allow-listed, and the fixture expects `deny` plus mutation-denial evidence. This is not a missing credential that should be repaired by adding an arbitrary capability reference. Installation belongs to a different admitted workflow.

**Safe next action:** classify parse failure, dispatch denial, or catalog failure using the response and request, not exit status alone. A chunk-store tool additionally needs a caller-supplied chunk root; it cannot infer one from request identity.

**Stop condition:** do not relabel a mutation as a read-only tool or interpret a receipt as permission to continue it elsewhere.

## Symptom: supposedly metadata-only inspection exposes more content

**Discriminating evidence:** registry view honors payload omission, but ledger view renders the ledger value even when `include_payload` is false. MCP redaction is a default boolean, not a mandatory policy gate. Hidden top-level selection and recursive confidentiality are also different operations.

**Safe next action:** stop exporting the result, preserve it only in an authorized location, and use list/search summaries for the narrower inspection question. Review the [exposure checklist](reviewing-catalog-exposure-and-redaction.md) before publishing an endpoint.

**Stop condition:** do not retry with raw rendering or treat a `visibility-check` marker as independent policy verification.

## Symptom: show rejects a generated summary or response

Two source-review discrepancies are relevant: the [summary builder](../../../src/catalog/parts/mod/p006/body.rs) emits twelve fields while the [summary recognizer](../../../src/catalog/parts/mod/p002/body.rs) expects eleven; the [MCP response builder](../../../src/catalog/parts/mcp/p001/body.rs) emits nine fields while its [summary recognizer](../../../src/catalog/parts/mcp/p000/body.rs) expects eight. These are source observations, not reproduced bugs.

Preserve the original canonical artifact and report the exact producer/consumer paths. Do not remove fields to satisfy the display helper: that would create a different value and identity. A display failure alone does not establish storage corruption.

## Sources

- [Handbook](../README.md)
- [Architecture](../../architecture.md)
- [Technical companion: evidence and authority separation](../../technical/foundations/evidence-and-authority-separation.md)
- [Catalog resolution and filtering](../../../src/catalog/parts/mod/p006/body.rs)
- [Catalog view branches](../../../src/catalog/parts/mod/p000/body.rs)
- [MCP dispatch and parsing](../../../src/catalog/parts/mcp/p000/body.rs)
- [MCP negative fixtures](../../../src/catalog/parts/mcp/tests/m000/p000/body.rs)
