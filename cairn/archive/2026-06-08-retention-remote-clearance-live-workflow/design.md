## Context

`retention-remote-gc-reconciliation` added per-remote clearance receipts and destructive admission requires every declared remote/peer to have a current passing clearance. The next gap is provenance of that clearance: a requester should be able to carry a canonical request to a peer, have the peer bind its retained/ref/revocation state into a response, and import only a scope-matching passing clearance locally.

## Decisions

### 1. Use transport-neutral request/response artifacts

**Choice:** Add `retention-remote-gc-clearance-request-v1` and `retention-remote-gc-clearance-response-v1` records. The request binds requester, peer, object, retention class, action, remote ref, policy, authority, and supporting evidence refs. The response embeds the exact request value and the exact `retention-remote-gc-clearance-v1` value produced by the peer.

**Rationale:** This can be carried by node-control live workflows, Iroh gossip/docs/blobs, offline bundles, or future protocols without changing retention semantics.

### 2. Import is the local safety gate

**Choice:** Add a local `retention-remote-gc-clearance-import-v1` receipt. Import recomputes the request, response, and clearance refs, checks peer/remote/object/action/policy/authority bindings, denies stale/revoked/retained/non-pass responses, and only stores the embedded clearance under the local retention store on pass.

**Rationale:** Destructive admission should consume local immutable evidence, but that evidence must first be proven to match the peer-produced response.

### 3. Clearance workflow remains evidence-only

**Choice:** Request, response, clearance, and import receipts remain deletion-safety evidence only. They do not grant authority, policy, resource, provenance, transport, execution, or source-gate trust.

**Rationale:** Peer clearance only proves remote retention reconciliation for the bound object/action. Local destructive authority and policy admission remain separate.

## Risks / Trade-offs

- The first implementation is transport-neutral and local-artifact based; a later node-control/Iroh live-send wrapper can carry these artifacts over an actual peer connection.
- Operators must preserve request/response pairs for auditability; import receipts bind both refs to make this explicit.
