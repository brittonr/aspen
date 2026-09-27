# Snapshot and restore authority

A vat snapshot is a representation of object state and authority relationships, not a transferable permission to revive everything it names. This article separates snapshot authority checks, schema-upgrade fixture evidence, and durable actor recovery. It assumes canonical Preserves references and actor-turn boundaries. The [architecture](../../architecture.md#vatobject-layer-goblins-inspired) and [addressable actor profile](../../addressable-actor-runtime.md) remain governing documents; navigation is in the [Technical companion](../README.md).

## Three boundaries that must not collapse

Snapshot content identity answers which represented bytes are under discussion. Snapshot admission answers whether the represented authority and visibility claims fit the supplied admitted set. Restore compatibility answers whether a selected representation can be interpreted under an explicit upgrade recipe. None of these alone proves durable storage, current authority, or successful runtime reconstruction.

The [snapshot fixture](../../../src/runtime/vat/parts/mod/p001/body.rs) constructs a `vat-snapshot-body-v1` containing a vat identifier, root/helper object descriptions, and reference collections. It hashes that body into `snapshot_ref`. The surrounding `vat-snapshot-v1` fixture also includes descriptors, predicate receipts, and diagnostics and has its own `fixture_ref`. These references identify different values. Treating the enclosing fixture hash as interchangeable with the snapshot-body hash loses the distinction between represented state and evidence about that state.

The fixture includes a far-object descriptor in its enclosing value even though the passing snapshot claims only admitted local authority. Mere presence of a descriptor in an artifact is therefore not a grant or a statement that the corresponding reference is restored live.

## Authority and visibility set constraints

The [snapshot-authority validator](../../../src/runtime/predicates/parts/mod/p004/body.rs) checks a canonical snapshot reference and canonical sorted collections for admitted authority, claimed authority, requested assertions, readable assertions, and redacted assertions. Its implemented relations are:

- claimed authority is a subset of admitted authority;
- readable assertions are a subset of admitted authority;
- readable and redacted assertions are disjoint;
- every requested assertion is either readable or redacted.

The last condition is coverage, not equality: the implementation does not require the readable/redacted union to equal the requested set. This distinction matters when reviewing an evidence projection. A description that omits a requested item entirely is different from one that explicitly records its redaction.

These are supplied-reference checks. The validator does not traverse the snapshot bytes to independently derive every held authority edge. The architecture describes the intended boundary as preventing claims outside the held/admitted graph; the executable local check establishes containment relative to the admitted collection supplied to it. Deriving that collection correctly remains an integration obligation, not something a canonical hash proves.

## A narrowly scoped source discrepancy

There is a concrete wording mismatch inside the receipt implementation. The [snapshot evaluator](../../../src/runtime/predicates/parts/mod/p003/body.rs) emits the check label `readable-assertions-subset-claimed`, but the [validator](../../../src/runtime/predicates/parts/mod/p004/body.rs) tests readable assertions against **admitted**, not **claimed**, references. The governing [architecture](../../architecture.md#vatobject-layer-goblins-inspired) states the broader held/admitted boundary; it does not resolve this internal label mismatch.

This article documents the observed predicate relation without declaring either wording a corrected contract. In particular, a receipt carrying that label should not be cited as proof that readable references are contained in the claimed set. The discrepancy is left for separate implementation/specification review; neither source is changed here.

## Worked example: safe redaction, unsafe resurrection

Consider an **illustrative** snapshot with admitted references `{root, helper}`. It claims `{helper}` and requests `{helper, remote}`. Setting readable to `{helper}` and redacted to `{remote}` satisfies the described authority and coverage relations. Setting readable to `{helper, remote}` fails because `remote` is not admitted. Omitting `remote` from both visibility collections fails because a requested assertion is uncovered.

Now imagine a reader attempting to revive the redacted remote object merely because its reference appears in the snapshot's review evidence. That inference is invalid. Redaction records a visibility disposition; it does not supply invocation authority, a live session, or restore admission. The same reasoning prevents a diagnostic artifact from quietly becoming a capability export channel.

## What the restore fixture actually demonstrates

`run_vat_restore_fixture` in the [restore implementation](../../../src/runtime/vat/parts/mod/p002/body.rs) constructs an old object under `schema:v1`, a corresponding object under `schema:v2`, and a `schema-rename` recipe. It constructs a passing receipt with a recipe and restored-object reference, plus a denial with `missing-compatible-upgrade-recipe` and neither of those optional references.

This is explicit fixture construction, not an inspected general migration engine executing arbitrary transformers. It supports the architecture's description of schema-rename and missing-recipe evidence, but should not be promoted into a claim that all application state migrations preserve behavior or authority. A changed canonical object reference identifies the changed representation; it does not certify semantic equivalence.

## Verification and recovery limits

Suggested verification uses the architecture's `molten test vat snapshot-fixture --out target/vat-snapshot.preserves` and `molten test vat restore-fixture --out target/vat-restore.preserves`, then reviews body references, fixture references, admission collections, and receipt decisions separately. These commands were not executed for this article.

For actual recovery, the [survival matrix](../../addressable-actor-runtime.md#survival-matrix) distinguishes durable selected checkpoints from runtime-only processes, streams, and sessions, and unsupported in-flight deltas. A checkpoint reference alone does not prove survival; matching bounded restore evidence is required. Before each wake-plan intent the shell rechecks current admission. A historical snapshot pass therefore establishes neither present activation authority nor production readiness.

## Sources

- [Technical companion](../README.md)
- [Architecture: vat/object layer](../../architecture.md#vatobject-layer-goblins-inspired)
- [Addressable actor recovery and survival](../../addressable-actor-runtime.md)
- [Snapshot fixture representation](../../../src/runtime/vat/parts/mod/p001/body.rs)
- [Snapshot-authority validator](../../../src/runtime/predicates/parts/mod/p004/body.rs)
- [Snapshot receipt labels](../../../src/runtime/predicates/parts/mod/p003/body.rs)
- [Schema-rename restore fixture](../../../src/runtime/vat/parts/mod/p002/body.rs)
