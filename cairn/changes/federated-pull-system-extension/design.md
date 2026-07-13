## Context

The accepted federation model has the correct receiver-driven trust boundary, but fixture signatures and local-root copying do not provide a live anti-entropy service. Federation must compose existing fabric capabilities without turning peer reachability or remote signatures into automatic local trust.

## Decisions

### 1. Federation is an optional system extension

**Choice:** A supervised extension owns peer configuration, announcement and inventory protocols, anti-entropy scheduling, missing-set sessions, conflict reporting, and local status assertions. Nodes without the extension retain normal local and cluster behavior.

**Rationale:** Federation is application-level eventual propagation, not a transport feature or global runtime invariant.

### 2. Discovery remains hint-only

**Choice:** Static configuration is the initial required discovery profile. Optional gossip, tracker, DHT, pkarr-style, endpoint, and catalog inputs produce canonical candidate locators with freshness and signer metadata but no import authority.

**Rationale:** Discovery quality and trust vary independently from content verification and policy.

### 3. All signed records use crypto adapters

**Choice:** Announcements, inventories, delegates, requests, and responses bind canonical Preserves payload refs, purpose/domain, signer public ref, key generation, and freshness. Signing and verification use opaque production adapter handles; fixture algorithms remain simulation-only.

**Rationale:** Federation cannot make production claims from shared-string or deterministic fixture signatures.

### 4. Pull planning is receiver-owned and bounded

**Choice:** The receiver selects peers, computes missing refs, chooses DAG/content strategies, enforces rate and resource budgets, and admits fetched artifacts locally. Remote push messages can update hint state only.

**Rationale:** Local policy must control bytes, state mutation, and trust.

### 5. Conflict and merge semantics stay domain-owned

**Choice:** Federation reports divergent refs, ancestry, signatures, and candidate policies. It does not select winners or merge application state unless the owning artifact or application extension provides an admitted merge policy.

**Rationale:** A global last-writer or pull-wins rule would leak application semantics into the fabric.

### 6. Anti-entropy evidence is scoped

**Choice:** Emit bounded evidence for peer/session admission, inventory verification, missing-set plans, fetch/verification, local admission, denial, conflicts, and status. Successful sync does not claim permanent convergence or remote correctness.

**Rationale:** Peer availability and inventories can change immediately after observation.

## Functional core / imperative shell split

- Pure core: signed-domain construction, inventory diff, candidate selection, missing-set and fetch planning, freshness/rate decisions, conflict classification, local-admission prerequisites, and receipt payloads.
- Shell: load signer handles, open transport sessions, schedule anti-entropy, perform DAG/content sync, invoke local admission, persist status, and emit operator evidence.

## Dependencies

- System-extension runtime.
- Production cryptographic identity adapters.
- Fabric transport, time, resource, observability, and simulation profiles.
- Content-store adapters and the DAG synchronization extension; the optional content-replication extension may consume imported availability afterward but is not required for federation.

## Risks / Trade-offs

- Malicious peers can advertise huge inventories. Bound records, pagination, query frequency, bytes, and concurrent sessions before allocation.
- Static peers simplify bootstrap but can stale. Record freshness and operator ownership; do not silently promote discovery mechanisms.
- Eventual convergence can be overclaimed. Scope receipts to exact inventories, sessions, and observation times.
