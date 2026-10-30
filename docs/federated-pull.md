# Federated receiver-owned pull

Federation is an optional manifest-installed system extension. Nodes without it
retain ordinary local and cluster behavior. The implementation does not turn
transport connectivity, discovery or a valid signature into artifact trust.

## Admission and records

The canonical federation manifest binds the admitted system-extension identity,
generation, configured peers, protocol, exact DAG binding and finite resource,
rate, freshness, local-policy and evidence profiles. Policy references must be
present in the already-admitted extension; wrapping an unrelated admitted
extension cannot introduce an arbitrary import policy.

The DAG owner exports `molten.extension.dag-sync@v1` with its `sync` operation and
canonical request/receipt schema identities. Normal fabric and system-tier
admission resolve it. Its implementation remains the existing `run_dag_sync`
shell and application-owned authority/resource/transport/content/progress ports.
The separately governed `dag-sync-fabric-admission-binding` prerequisite records
this contract; archive status does not admit a runtime configuration.

Announcements, inventories, delegates, fetch requests, responses and detached
receipts use separate canonical signature domains. Production operations use
opaque Ed25519 handles through the existing cryptographic identity adapter.
Public-key refs, independent key generations, verifier context, canonical payload
refs and freshness are bound. The former fixture-signature/local-root-copying
API is confined to test fixtures and is not the production federation surface.

## Receiver effect order

1. Observe static configured peers and optional hint-only locators.
2. Verify the signed inventory and derive the missing set on the receiver.
3. Reject stale, oversized, unavailable, unsolicited, rate-limited or over-budget
   work before starting a trusted import.
4. Dispatch a current-generation/incarnation typed effect through the supervised
   extension's admitted DAG binding.
5. Send a signed request over actual cross-process Iroh transport. Accept only a
   signed response bound to that exact request and requested object.
6. Verify canonical object identity and the content manifest through the admitted
   content adapter in a capability-rooted quarantine store.
7. Require complete correlated DAG verification and all local schema, policy,
   capability, provenance, resource, retention and import gates for the exact
   head/session/generation.
8. Preview the commit transition, persist intent, mutate the capability-rooted
   immutable ledger, then persist completion.

A failure after the protected effect is uncertainty requiring local readback,
not a policy denial or authorization for an automatic retry. Partial DAG progress
is not trusted registry import. Quarantine availability grants no install,
execution, publication, pin or merge authority.

## Policy, conflict and bounds

The local policy adapter consumes a manifest-admitted local capability proofset,
validates the actual boundary schema, restricts allowed artifact heads and signed
origins, reserves cumulative bytes and evaluates explicit retention/provenance/
resource rules. Referenced rule identities must match the evaluated rules;
reference presence alone is not approval. A remote delegate remains a hint and
never becomes a local capability token automatically.

Divergent logical names preserve both candidate refs and produce canonical
conflict evidence with ancestry, signer, freshness and policy. Federation does
not implement pull-wins, last-writer or an automatic merge policy.

Bounds include inventory entries/bytes, query interval, concurrent sessions,
wire bytes, nodes, chunks, retries, physical session deadline, logical session
window, freshness, evidence capacity and retained diagnostics. Logical and
physical time remain separate. Evidence capacity is planned before transfer.
Status contains bounded refs and diagnostics, not payloads, credentials or raw
keys, and never claims permanent convergence or remote correctness.

Periodic polling uses an admitted time adapter. Capability-rooted checkpoints
retain active plans, progress and status sequence; caller-owned current
supervision state must be activated or recovered before service startup. Restart
does not recreate an absent host implicitly or reset persisted query limits.

## Runnable bounded evidence

The public fixture entrypoints are `liveproof::loopback`,
`liveproof::distinct_process`, `liveproof::role` and
`liveproof::verify_detached`. Distinct-process execution launches the real Molten
CLI twice with separate role roots and verifies different OS process identities.
Files exchange endpoint/identity metadata only; artifact bytes travel over Iroh.
Filesystem copying is not labeled live networking.

After building/installing the current executable, use fresh owner-selected roots:

```sh
molten federation live-loopback --run-root target/federation-loopback \
  --dag-conformance-ref "$DAG_CONFORMANCE_REF"
molten federation distinct-process --run-root target/federation-process \
  --process-binary "$(command -v molten)" --dag-conformance-ref "$DAG_CONFORMANCE_REF"
molten federation verify-detached \
  --receiver-root target/federation-process/receiver \
  --trusted-context-ref "$TRUSTED_CONTEXT_REF"
```

`DAG_CONFORMANCE_REF` must identify genuine independently reviewed current DAG
owner evidence. It is a mandatory trusted admission input, with no generated
default. A source-file hash, an archive entry or an unexecuted fixture label is
not passing conformance evidence. The fixture persists the supplied ref through
its capability-rooted role handoff; it does not manufacture owner approval.

For actual malicious-wire cases, pass `--fault corrupt-signature`, `wrong-key`,
`wrong-domain`, `corrupt-content` or `unsolicited-push` to either run command with
a separate fresh root. A successful negative fixture requires the real boundary
denial and absence of the artifact in the receiver registry, not merely an error
line or an omitted completion receipt.

Obtain `TRUSTED_CONTEXT_REF` through the trusted local run result or another
independently authenticated channel. Do not trust a replacement key/context
merely because it is embedded in a bundle. Public detached leaves are
`receiver/receipts/detached-bundle.preserves` and
`receiver/receipts/public-context.preserves`. The offline verifier checks exact
signed inventory/request/response, transport, DAG, content, seven local admission
gates, import, actual registry readback and exact supervised manifest/binding/
callback/effect-completion provenance without network access.

The deterministic replay path uses the same receiver planner, transitions,
session identity and checkpoint codec, with actual verified recorded inputs.
Named campaigns distinguish planner faults from genuine cryptographic/content
boundary denials; a synthetic rejection flag cannot substitute for those effects.
The installed-extension profile pins all seven core/schema/port identities and
the maintained invariant evaluator, then dispatches its exact verified input
through the shared typed installed-workload scheduler. The resulting proof
contains the immutable profile and actual named campaign/live-plan comparison;
it is not metadata-only admission or an arbitrary callback exemption.

These commands describe runnable interfaces, not an execution report. No command,
test or validation result is asserted by this document. Runtime receipts from the
actual invocation determine what was exercised and remain bounded fixture proof,
not deployment or permanent-convergence admission.
