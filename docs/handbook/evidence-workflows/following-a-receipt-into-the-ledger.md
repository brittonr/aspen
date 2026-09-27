# Following a receipt into the ledger

Mode: Walkthrough

This source-only walkthrough follows `chain_ledger_append_stores_links_indexes_heads_and_receipts`, a checked-in Rust fixture, from its first payload to the stored append receipt and the next chain head. It is useful when a handoff includes several BLAKE3 refs and you need to distinguish the subject, its continuity link, and the receipt recording that link's admission. Return to the [Handbook](../README.md) for other workflows; the [technical companion](../../technical/foundations/evidence-and-authority-separation.md) explains why none of these objects grants authority.

The fixture and implementation were inspected, not executed for this document. This is not a live-node tutorial or a claim that an existing operator ledger contains the fixture objects. Its inputs are generated inside the test, so there are deliberately no invented runnable hashes or output transcripts.

## 1. Identify the exact fixture inputs

Open the [chain fixture](../../../src/evidence/parts/chain/tests/m000/p000/body.rs), specifically `chain_ledger_append_stores_links_indexes_heads_and_receipts`. It creates an isolated root and a scope with `scope = evidence-ledger`, `id = node-a`, and `epoch = epoch-1`.

The first input is `stored_payload(&root, "payload-a")`. Its [helper](../../../src/evidence/parts/chain/tests/m000/p005/body.rs) constructs a `test-payload` record and imports it through the ledger. This is synthetic test evidence, not a production gate receipt. That distinction matters: the fixture exercises continuity and storage, not the truth of a workload claim.

**Observable boundary:** the helper returns a payload descriptor containing the imported artifact ref. The payload already exists in the ledger before chain append begins. Merely naming a ref in a link does not materialize its bytes.

## 2. Separate payload identity from link identity

`ChainLinkInput::genesis` adds the scope, sequence zero, no previous link, producer information, and the genesis predicate/check records. The fixture serializes that input with `chain_link_value` and parses it back into a `ChainLink`.

The link has its own canonical Preserves+BLAKE3 identity. It names the payload without rewriting it. The neighboring test `chain_link_preserves_payload_ref_without_rewriting_payload` explicitly checks that the link ref differs from the payload ref and that hashing the payload again gives its original ref. Rust struct layout, textual whitespace, and the eventual storage pathname are not alternative identity definitions.

**Observable boundary:** distinguish `genesis_link.link_ref` from `genesis_link.payload.artifact_ref` in review notes. Substituting one for the other changes the object being requested.

## 3. Follow append through storage

The [append implementation](../../../src/evidence/parts/chain/p009/body.rs) parses the link, builds the existing chain index, and reads the payload through `ledger::read_artifact`. An unavailable or hash-mismatched payload stops this path. For a new link, it obtains the prior head through the chain admission path and imports the link itself.

The [ledger import implementation](../../../src/ledger/parts/mod/p000/body.rs) derives canonical bytes and a canonical ref. An existing content file is parsed and rehashed before reuse; a missing file is written. Content naming is handled by the ledger rather than by an operator fabricating a pathname.

**Observable boundary:** import establishes content-addressed storage of a value. It does not validate every semantic claim carried by that value. Chain append adds its own continuity checks above that storage boundary.

## 4. Find the receipt that is actually stored

Append obtains a predicate receipt ref, constructs `chain-append-receipt-v1`, hashes it, and explicitly imports that receipt into the ledger. The returned `ChainAppend` therefore carries separate `link_ref`, `payload_ref`, `predicate_receipt_ref`, and `receipt_ref` fields, plus before/after heads.

The fixture reads both the link and append receipt back by ref and compares them with their original values. It also reads the predicate receipt and checks the genesis predicate. This is stronger evidence than seeing a summary that says an append occurred: it demonstrates the intended objects are individually addressable in the checked-in test.

Do not generalize this storage behavior to every receipt-returning function. Ordinary `import_artifact` returns an import receipt value; it does not itself recursively import that receipt. In this path, append makes the extra receipt import explicit.

## 5. Observe the second link and derived index

The fixture creates `payload-b`, builds a child with `ChainLinkInput::append`, and appends it. It expects the previous genesis ref as `head_before` and the second link ref as `head_after`. Rebuilding the index yields the second link as the scope's head, the child under its parent, and separate entries for sequences zero and one.

These are scoped continuity observations, not global actor-message ordering, consensus, fork choice, or exactly-once external effects. The index is a view derived from ledger objects; its existence does not authorize an operation on the payload.

## 6. Carry the failure boundary into a handoff

The adjacent negative fixture checks a missing payload, a sequence gap, and an unexpected fork. A reviewer can name those source cases as unexecuted regression coverage, but must not report them as fresh passing runs. Preserve the rejected link and diagnostic context; do not “repair” continuity by deleting ledger entries or rewriting sequence numbers.

A useful handoff records the scope tuple, payload ref, link ref, append receipt ref, predicate receipt ref, before/after heads, and the fixture-only status. The [proof workflow](../../proof-workflow.md) additionally asks for explicit non-claims and positive/negative evidence. Live admission and current authority remain outside this walkthrough.

## Sources

- [Handbook](../README.md)
- [Evidence and authority separation](../../technical/foundations/evidence-and-authority-separation.md)
- [Proof workflow](../../proof-workflow.md)
- [Checked-in chain fixture](../../../src/evidence/parts/chain/tests/m000/p000/body.rs)
- [Fixture payload helper](../../../src/evidence/parts/chain/tests/m000/p005/body.rs)
- [Chain append implementation](../../../src/evidence/parts/chain/p009/body.rs)
- [Ledger import and readback](../../../src/ledger/parts/mod/p000/body.rs)
