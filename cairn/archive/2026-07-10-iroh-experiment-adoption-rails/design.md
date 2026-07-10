## Context

The upstream `iroh-experiments` repository provides five relevant experiments:

- `content-discovery`: tracker-backed signed content announcements, queries, partial/complete claims, and cheap verification probes.
- `iroh-dag-sync`: deterministic traversal descriptors, inline-data policy, non-BLAKE3 CID mapping, and deep DAG sync over Iroh streams.
- `iroh-pkarr-naming-system`: pkarr-backed mutable pointers from public keys to current Iroh content refs.
- `h3-iroh`: HTTP/3 request/response over Iroh connections.
- `iroh-s3-bao-store`: remote byte storage with content-addressed verification and outboard metadata.

Molten already has canonical content refs, chunk manifests, remote artifact sync, federated pull sync, Iroh live transport receipts, operator gateway readback, and strict non-authority boundaries for transport evidence. The design goal is to adopt the useful shapes while preserving those boundaries.

## Design

### Data flow

```text
tracker / pkarr / static peer / catalog hint
  -> content-locator-v1 evidence
  -> receiver computes missing set from local registry and manifests
  -> deterministic traversal descriptor or chunk fetch plan
  -> admitted Iroh/blob/gateway fetch shell
  -> streaming hash and manifest verification
  -> local policy, capability, provenance, source-gate, resource admission
  -> import/readback receipt or deny receipt
```

Discovery and naming never import content by themselves. The receiver chooses what to fetch, verifies every fetched byte against canonical content identity, and only then asks local admission gates to install, expose, or execute the content.

### Locator evidence

Introduce locator records as evidence-only hints:

- `content-locator-announcement-v1` for signed peer claims about a content ref or manifest ref.
- `content-locator-query-v1` for receiver query criteria, such as complete-only, verified-only, content kind, and freshness policy.
- `content-locator-result-v1` for tracker, pkarr, static, catalog, or peer-observed candidates.
- `content-locator-probe-receipt-v1` for bounded probes that check peer reachability, declared size, or sampled chunk availability.

An announcement may say a peer claims partial or complete availability, but it is not proof of possession. A probe improves diagnostics and fetch planning, but cannot replace full hash verification or local admission.

### Optional pkarr naming

Pkarr-style records are mutable public-key-indexed pointers. Molten should support them as optional discovery inputs for public locator bootstrap, tracker discovery, or latest-pointer readback. Resolution produces locator evidence with freshness, signer, key, and resolved-ref bindings. It does not produce authority, provenance, source-gate, or import trust.

### Deterministic traversal descriptors

Adopt the `iroh-dag-sync` principle that sender and receiver execute the same deterministic traversal. In Molten terms, a traversal descriptor should name:

- traversal kind, such as artifact closure, chunk manifest tree, job DAG output closure, sequence, or policy-defined traversal;
- root refs and optional already-visited refs;
- deterministic order;
- filters, such as stem-only, leaf-only, kind exclusion, or stage selection;
- inline policy, such as include all, metadata-only, stem-only, leaf-only, or none;
- resource and replay bounds;
- policy refs and evidence refs.

The pure core computes traversal plans, missing sets, and expected refs from in-memory registry summaries. The shell performs network reads, writes fetched bytes, and records receipts.

### Non-BLAKE3 interop

Molten-owned identity remains BLAKE3 over canonical Preserves or verified bytes. When interoperating with IPFS/CID data that uses another hash algorithm, the non-BLAKE3 digest is an external compatibility claim. Molten may store a mapping from external CID digest to BLAKE3 content ref only after validating the bytes against both the external digest and the Molten content ref.

### Chunk-store sync strategies

Chunk manifests allow deterministic traversal to be applied without importing a generic IPFS DAG model. Molten should support:

- stem-first sync for metadata and branch nodes before large leaves;
- partitioned leaf fetch across multiple peers after stem verification;
- resumable fetch based on local missing chunks;
- range readback that verifies only relevant chunks before emitting bytes;
- remote byte-source hints whose locations remain separate from identity.

### Remote byte-source and outboard readback

The S3/outboard idea is useful for public evidence mirrors and large external resources: bytes may remain in S3, HTTP, or another remote source while Molten stores verification metadata and location hints. This must be readback-only until the fetched ranges are verified against canonical refs. Location hints cannot pin, retain, import, delete, execute, or expose hidden content without subsystem gates.

### HTTP/3-over-Iroh adapter

HTTP/3-over-Iroh is useful for operator UX and compatibility with HTTP tooling, but it should not become Molten's internal protocol. The adapter shell may translate admitted read-only HTTP requests into canonical operator gateway read/range/index requests and return rendered responses. The normative evidence remains the Molten gateway receipt. HTTP route names, headers, sessions, endpoint identity, and TLS/QUIC state do not grant authority or replace Preserves envelopes.

### Functional core and shell split

Pure cores:

- validate locator records and query criteria;
- plan deterministic traversals and missing sets;
- validate inline policies and filters;
- validate external-digest to BLAKE3 mapping claims from byte summaries;
- plan readback decisions and deny unverified exposure.

Imperative shells:

- talk to trackers, pkarr, Iroh, HTTP/3, S3, or local filesystem stores;
- stream bytes and write local caches;
- invoke capability, policy, provenance, source-gate, resource, and retention checks;
- persist receipts and render diagnostics.

### Non-goals

- No dependency on upstream `iroh-experiments` crates as stable APIs.
- No adoption of postcard or RON as a normative Molten boundary.
- No tracker, pkarr, HTTP/3, S3, endpoint, or topic evidence as authority.
- No global mutable namespace for artifacts.
- No proof of full possession from random probes.
- No runtime Nickel evaluation or ambient network behavior in pure cores.
