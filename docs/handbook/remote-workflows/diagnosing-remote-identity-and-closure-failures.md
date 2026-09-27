# Diagnosing remote identity and closure failures

Mode: Troubleshooting

A remote workflow can fail before reading bytes, after verifying identity, during policy filtering, or after earlier local effects. First preserve the ticket or inventory, expected references, selected roots, policy, session generation, and any destination evidence. Do not delete state, change trust roots to match a peer, or repeat an uncertain import merely to obtain a cleaner receipt.

This page is source-checked and was not runtime-executed for this batch. It provides diagnostic decision paths, not terminal commands. Return to the [Handbook](../README.md); consult the [DAG closure companion](../../technical/replication/bounded-dag-closure-exchange.md) for the distinction between graph reachability and requested objects.

## Ticket rejected before content is read

**Symptom:** reproduction fetch rejects the ticket prefix or reports that the advertised bundle differs from the expected bundle.

**Discriminating evidence:** [the local exchange helper](../../../src/iroh/parts/exchange/p000/body.rs) accepts `iroh-local:` for this path and compares `expected_bundle_ref`, when provided, before reading the blob. A wrong-prefix ticket is not evidence that a live peer is unreachable. An expected-reference mismatch is not a content-store corruption diagnosis.

**Safe next action:** establish which exchange API produced the ticket and where the independently expected reference came from. Keep both values in the incident record. Resolve mismatched artifact selection with the producer rather than dropping the expected-reference check.

**Stop condition:** there is no independently accepted identity or the ticket belongs to a different transport surface. Do not reinterpret it by manually rewriting its prefix.

## Bytes are present but identity fails

**Symptom:** canonical parsing fails, a fetched bundle hashes to a different reference, or remote content bytes fail their content-reference comparison.

**Discriminating evidence:** bundle fetch parses canonical bytes and compares the canonical bundle hash with the advertised reference. [Dataspace content checking](../../../src/remote/parts/dataspace/p003/body.rs) reads each referenced blob and hashes its bytes. These are different checks: canonical artifacts have an encoding boundary, while referenced content bytes are verified as bytes.

**Safe next action:** preserve the received bytes and advertised metadata separately in an authorized diagnostic location. Determine whether the wrong object was selected, bytes changed, or text representation was confused with canonical binary representation. Obtain a fresh independently identified object through the admitted path only after that distinction is understood.

**Stop condition:** the proposed fix is to relabel the received bytes with the expected reference, replace verification with a transport receipt, or treat a Rust debug representation as canonical input.

## “All refs returned” still yields a traversal denial

**Symptom:** response validation reports missing or unrequested references, or deterministic order mismatch.

**Discriminating evidence:** [remote traversal response validation](../../../src/remote/parts/dataspace/p005/body.rs) compares the response with `plan.fetch_refs`, including exact ordering. The expected set excludes references already covered by the plan's inventory. Sending an extra object “for convenience” can therefore fail even when all requested objects are present.

**Worked failure case:** let a captured plan request actual references A then B, where A and B are explanatory names, not runnable content addresses. A response B then A contains the same set but fails the sequence comparison. A response A, B, C also introduces an unrequested object. Diagnose the producer's response construction; do not expand receiver selection after the fact to make the response pass.

Inline data cannot repair a denied plan when the response helper sees a nonempty inline list. More broadly, this helper checks reference lists; it does not fetch or verify all graph bytes. A response receipt is not a substitute for the plan's own decision or complete DAG admission.

**Stop condition:** only a response receipt is available, without its selected plan and inventory. Preserve it as incomplete evidence rather than claiming closure.

## Resume fails after reassignment or a changed request

**Symptom:** a previously useful progress record is rejected when peers, roots, schema references, strategy, policy, epoch, or generation change.

**Discriminating evidence:** the [DAG contract](../../dag-sync.md) binds these facts into progress. Peer reassignment requires a new traversal epoch. Completion under leaf-only selection is also not completion under full selection.

**Safe next action:** compare the complete old and new request contexts, not just the root reference. Obtain a new receiver-owned plan for the changed context. Verified content may inform inventory; old progress must not be relabeled as belonging to a new peer assignment.

**Stop condition:** the only suggested recovery discards the fencing fields or claims global convergence from local availability.

## Inventory pull fails after importing some resources

**Symptom:** the pull has both `imported_refs` and `denied_refs`, or an import operation returns an error after processing began.

**Discriminating evidence:** [the federation loop](../../../src/federation/parts/mod/p002/body.rs) processes resources individually. Type/delegate checks precede duplicate skipping; bounds, source readability, hash, and actual artifact kind influence later decisions. An overall `fail` is not an atomic rollback signal.

**Safe next action:** reconcile actual destination artifacts with retained result evidence before authorizing another operation. Check the original policy rather than weakening it. A signature mismatch should be investigated against expected purpose, trust root, algorithm, and supplied key context in [signature verification](../../../src/federation/parts/mod/p003/body.rs), not “fixed” by trusting the advertised root.

**Stop condition:** earlier effects cannot be distinguished from pending work, or recovery requires unrestricted delegation or policy bypass.

## Delivery succeeded but the assertion is absent

Inspect the declared receiving session before investigating transport retries. Unknown or closed sessions are denied; session-aware replay remains diagnostic for those owners. Follow the [session inspection procedure](inspecting-a-remote-dataspace-session.md). Successful delivery does not establish a still-live owner, current capability authority, or exactly-once application.

## Sources

- [Handbook](../README.md)
- [Architecture and canonical boundaries](../../architecture.md)
- [DAG synchronization contract](../../dag-sync.md)
- [Bounded DAG closure companion](../../technical/replication/bounded-dag-closure-exchange.md)
- [Exchange verification and output ordering](../../../src/iroh/parts/exchange/p000/body.rs)
- [Traversal plan and response validation](../../../src/remote/parts/dataspace/p005/body.rs)
- [Federation pull processing](../../../src/federation/parts/mod/p002/body.rs)
- [Session denial and ownership](../../../src/remote/parts/dataspace/p006/body.rs)
