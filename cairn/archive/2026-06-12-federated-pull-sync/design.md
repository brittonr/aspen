## Context

Aspen federation emphasizes sovereignty, pull-based sync, and layered verification. Molten needs similar principles for sharing artifacts and evidence across peers while keeping Syndicate/SAM local interaction semantics. Federation synchronizes content and metadata; it does not make every remote assertion local truth by default.

## Goals

- Let independent Molten nodes/clusters share selected resources without central authority.
- Keep sync pull-based and receiver-admitted.
- Verify origin signatures, delegate signatures, content hashes, and policy refs.
- Support application-level sync protocols over shared primitives.
- Avoid global consistency claims across partitions.
- Produce receipts for discovery, query, fetch, verification, admission, denial, and merge/import.

## Non-Goals

- Do not make remote clusters part of one global Raft group by default.
- Do not let peers push state into a receiver without request/admission.
- Do not treat remote catalog visibility as data access authority.
- Do not replicate ordinary local actor messages as federation semantics.

## Federation model

Each node/cluster has local namespaces, policies, and authority roots. Federation resources are advertised as signed announcements or queried inventories. A receiver chooses resources to fetch, verifies them, then imports them into local registry/docs/storage surfaces only after policy admission.

## Verification layers

Remote data should pass through:

1. Origin/node/cluster signature over announcement or inventory.
2. Delegate/capability signature for resource-specific authority where applicable.
3. Content/chunk hash verification for fetched bytes.
4. Local policy admission before use or publication into local dataspaces/catalogs.

## Resource types

Initial federated resources may include:

- artifact manifests and payloads,
- chunk manifests and chunks,
- docs/catalog metadata,
- receipts and provenance records,
- transcript outputs,
- protocol or schema artifacts,
- application-defined immutable objects.

Mutable state should sync through application-defined protocols with explicit conflict/merge semantics.

## Discovery and announcements

Discovery can use configured peers first, then Iroh/gossip/DHT-like mechanisms later. Announcements are hints and must be verified. Rate limits and trust levels protect against spam.

## Pull workflow

```text
discover peer/announcement
query inventory or resource state
compute missing set
fetch manifests/chunks/artifacts
verify signatures and content hashes
apply local policy
import or deny with receipt
```

## Integration with actors

Federated sync can be represented by service actors that assert imported resources, available peers, sync status, failures, or new catalog entries into the local dataspace. Ordinary actor traffic remains local/transport semantics, not global federation.

## Open Questions

- Should first discovery be static peer config only?
- Which signature format and delegate model should be required initially?
- How should mutable Iroh docs imports represent conflicts and local policy denials?
