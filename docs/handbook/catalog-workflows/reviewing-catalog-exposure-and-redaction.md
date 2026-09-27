# Reviewing catalog exposure and redaction

Mode: Review checklist

Use this checklist before exposing catalog results to a new audience, embedding the MCP-style dispatcher behind another interface, or publishing discovery evidence. Review the actual composition, not just the read-only tool names. The governing [architecture](../../architecture.md) separates admitted effects from canonical evidence; the [technical companion](../../technical/foundations/evidence-and-authority-separation.md) explains the authority consequences.

This is a source-review checklist, not an attestation. No tests, live endpoint, or CLI recipe were executed for it. Record each answer as accepted with evidence, rejected, or not established. An unestablished confidentiality or authority boundary is a reason to withhold exposure, not to assume the default is sufficient.

## 1. Establish the audience and root boundary

- [ ] **Who may observe this corpus?** Identify the principal and governing policy outside the catalog query itself. [Visibility validation](../../../src/catalog/parts/mod/p008/body.rs) validates reference shapes; it does not evaluate current policy, revocation, or a caller's entitlement to a store.
- [ ] **Who chooses registry, ledger, and chunk roots?** Require evidence that the exposing adapter supplies only authorized roots. The MCP request does not embed these host paths as canonical authority. A root passed by a trusted CLI user is not automatically safe to accept from a remote caller.
- [ ] **Who fixes the visibility context?** Inspect whether a consumer can omit `hidden-ref`, replace a profile reference, or provide arbitrary policy/capability references. [Argument parsing](../../../src/catalog/parts/mcp/p002/body.rs) builds visibility from request arguments; those values do not become independent grants.

Acceptance evidence should identify the actual wrapper or adapter enforcing these decisions. The inspected local dispatcher is not proof that such a wrapper exists in a deployment.

## 2. Check every output representation

- [ ] **Are both registry and ledger views reviewed?** The [view implementation](../../../src/catalog/parts/mod/p000/body.rs) omits a registry payload when requested, but renders the ledger value in its fallback branch regardless of `include_payload`. A UI promising “metadata only” needs evidence specific to its chosen branch.
- [ ] **Are summaries and search material reviewed separately?** [Search text construction](../../../src/catalog/parts/mod/p006/body.rs) combines summary text, artifact metadata, and redacted payload text. Do not assume redacting the last component proves confidentiality of the other components.
- [ ] **Are responses, receipts, diagnostics, and output files included?** A sensitive reference may appear outside an item body. The [CLI IO helper](../../../src/cli/core/catalog/io.rs) prints receipts and optionally writes files; review terminal capture and file destinations as well as the nominal response.

Accept only a clearly stated exposure contract: for example, suppression of an artifact as a result item is a narrower promise than suppressing every mention of its reference.

## 3. Separate hidden items from hidden relationships

- [ ] **What exactly does `hidden-ref` remove?** [Collection](../../../src/catalog/parts/mod/p002/body.rs) skips objects whose own references are hidden. [Registry summary construction](../../../src/catalog/parts/mod/p003/body.rs) separately carries dependency and dependent references. Require review evidence for relationships, names, evidence references, and classifications, not merely top-level result counts.
- [ ] **Are direct lookup and graph paths covered?** A full hidden reference is rejected by resolution, but each operation has its own traversal and rendering path. Evidence for search alone does not establish the behavior of view, impact, receipts, or chunk discovery.
- [ ] **Are candidate lists treated as potentially sensitive?** Short-ID ambiguity is relative to the visible registry/ledger corpus. Candidate lists and diagnostics should not be published to a broader audience than the underlying references.

**Worked review case:** suppose a visible document depends on a hidden schema artifact. The hidden schema may be absent from top-level list items while its reference remains in the document's dependency metadata. The builders support this source-level concern; it is not a reproduced leak from this documentation work. Reject a blanket “hidden references never appear” claim unless evidence covers that relationship case. Do not fix the review by hiding arbitrary extra objects or changing storage.

## 4. Evaluate the redaction guarantee actually implemented

- [ ] **Can the caller request raw rendering?** The [MCP view adapter](../../../src/catalog/parts/mcp/p003/body.rs) reads `redacted` with a true default but accepts a supplied false value. A default is not authorization for reveal and is not mandatory enforcement.
- [ ] **Are secret markers structurally present?** The [redaction implementation](../../../src/secrets/parts/mod/p010/body.rs) returns the value unchanged if no secret marker is found; otherwise it produces a redaction marker and associated transform evidence. It is not a universal detector for sensitive unlabelled strings.
- [ ] **Does the review preserve identity distinctions?** The source value, transformed marker, summary, query result, and receipt have different canonical roles. A content commitment does not grant access to the committed content.

The [MCP redaction fixture](../../../src/catalog/parts/mcp/tests/m000/p000/body.rs) checks a payload with a structural `secret` marker and default-redacted view. Cite that as fixture coverage, not comprehensive confidentiality or production isolation.

## 5. Prevent evidence from becoming admission

- [ ] **Is the allow-list boundary intact?** Unlisted tools receive deny responses. Confirm that the exposing shell cannot reinterpret a denied tool as an execution or mutation request.
- [ ] **Does the consumer inspect decisions?** A constructed deny call can still be a successful CLI operation. Process success, receipt existence, and passing search completion are insufficient admission signals.
- [ ] **Are check markers interpreted conservatively?** In [MCP check parsing](../../../src/catalog/parts/mcp/p002/body.rs), statuses are validated as `pass` or `fail`, but the returned collection retains names, and `require_check` checks name presence. Do not treat required-marker presence as an independent proof that its status passed.
- [ ] **Are stronger claims separately justified?** Catalog presence does not establish provenance trust, execution rights, retention clearance, current authority, replay correctness, or release readiness.

The final review record should list the exposed operations, audience, root-selection owner, field-level confidentiality promise, applicable fixture evidence, and unresolved source observations. Withhold claims beyond that record. No catalog receipt can fill a missing admission boundary.

## Sources

- [Handbook](../README.md)
- [Architecture](../../architecture.md)
- [Technical companion: evidence and authority separation](../../technical/foundations/evidence-and-authority-separation.md)
- [Catalog visibility validation](../../../src/catalog/parts/mod/p008/body.rs)
- [Summary and search representations](../../../src/catalog/parts/mod/p006/body.rs)
- [MCP parsing and check markers](../../../src/catalog/parts/mcp/p002/body.rs)
- [MCP view argument handling](../../../src/catalog/parts/mcp/p003/body.rs)
- [Structural redaction implementation](../../../src/secrets/parts/mod/p010/body.rs)
- [MCP redaction and denial fixtures](../../../src/catalog/parts/mcp/tests/m000/p000/body.rs)
