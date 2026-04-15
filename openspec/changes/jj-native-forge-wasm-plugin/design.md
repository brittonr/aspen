## Context

Aspen Forge is Git-first today. Git clients reach Forge through `git-remote-aspen`, Git bridge RPCs, Git refs, and SHA-1/BLAKE3 translation layers. That works for compatibility, but it does not preserve Jujutsu-native concepts such as stable change IDs, bookmark semantics, and conflict-preserving object forms. Aspen already has the pieces needed for a better fit: BLAKE3-addressed blob storage, Raft-backed metadata, iroh QUIC transport, and a WASM plugin model that can isolate repo-domain behavior.

This change adds JJ-native access without weakening Aspen's existing constraints:

- no HTTP transport; JJ traffic still uses iroh QUIC
- no unbounded transfers; large syncs must stay chunked and bounded
- no Git regression; Git bridge remains a first-class path
- no hidden mutable state; repo metadata and bookmark updates still flow through Raft

## Goals / Non-Goals

**Goals:**

- Preserve JJ-native repository state in Forge without forcing it through Git SHA-1 translation.
- Expose a JJ-native clone/fetch/push/bookmark-sync path over Aspen transport.
- Let repositories advertise `git`, `jj`, or both backends in one repo record.
- Keep JJ-specific behavior isolated in a Forge-adjacent WASM plugin.
- Bound large JJ transfers with the same transport discipline Aspen already applies elsewhere.

**Non-Goals:**

- Replace Git bridge or remove Git repository support.
- Recreate JJ's full local operation-log model on the server in the first cut.
- Auto-convert every existing Git repo into a JJ-enabled repo.
- Add raw TCP or HTTP endpoints for JJ clients.

## Decisions

### 1. Repository metadata grows a backend manifest

**Choice:** Extend Forge repo metadata so each repo declares which backends are enabled: `git`, `jj`, or both.

**Rationale:** Repo-level capability advertisement gives clients an explicit contract before they attempt sync. It also makes mixed Git/JJ hosting possible without creating separate top-level repository types.

**Alternative:** Split Git repos and JJ repos into separate resource kinds. Rejected because it duplicates repo management, federation, and authorization paths that should stay shared.

### 2. JJ data stays native and BLAKE3-addressed

**Choice:** Store JJ objects as native JJ payloads in Forge's blob store, with Raft-backed indexes for repo reachability, change IDs, and bookmarks.

**Rationale:** Aspen already prefers BLAKE3-addressed content. Native JJ storage avoids SHA-1 mapping churn and preserves JJ identity directly.

**Alternative:** Encode JJ state indirectly through Git objects plus sidecar metadata. Rejected because it keeps the lossy Git bridge in the middle and makes JJ identity reconstruction fragile.

### 3. JJ behavior lives in a dedicated WASM plugin

**Choice:** Add a JJ-native Forge WASM plugin that claims JJ request families, JJ protocol identifiers, and JJ storage prefixes.

**Rationale:** This follows Aspen's plugin split: shared control-plane pieces remain native, while repo-domain behavior can evolve inside a sandboxed plugin. It also keeps JJ logic unloadable/reloadable on the same operational path as other Forge plugins.

**Alternative:** Put JJ support directly into the native Forge handler. Rejected because it raises blast radius, couples JJ evolution to core handler releases, and weakens the plugin story Aspen already uses for Forge-adjacent features.

### 3a. JJ state uses fixed repo-scoped prefixes

**Choice:** JJ repo state uses explicit repo-scoped prefixes so storage isolation, deletion, and host-enforced plugin permissions all line up on the same key layout. The first-cut layout is:

- repo backend manifest: existing Forge repo metadata record extended with enabled backends, defaulting missing manifests to `git`
- JJ bookmarks: `forge:jj:bookmark:{repo_id}:{bookmark_name}`
- JJ change-id heads: `forge:jj:change:{repo_id}:{change_id}`
- JJ reachability and object membership: `forge:jj:reach:{repo_id}:{object_hash}`
- JJ repo discovery metadata: `forge:jj:discover:{repo_id}`
- staged session metadata: `forge:jj:stage:{repo_id}:{session_id}:meta`
- staged session object references: `forge:jj:stage:{repo_id}:{session_id}:obj:{object_hash}`

The JJ plugin gets host-enforced read/write access only to the `forge:jj:bookmark:{repo_id}:`, `forge:jj:change:{repo_id}:`, `forge:jj:reach:{repo_id}:`, and `forge:jj:stage:{repo_id}:` prefixes for the repo/session it is serving. Repo metadata and discovery assembly stay in native Forge admission.

**Rationale:** Prefixes must be concrete for runtime permission checks, delete tombstoning, and dual-backend namespace separation to be reviewable and testable.

**Alternative:** Leave prefixes implicit in implementation code. Rejected because deletion, access control, and coexistence semantics become impossible to verify from the design.

### 4. JJ transport is separate from `git-remote-aspen`

**Choice:** Add a JJ-native remote path with dedicated request families and an explicit standalone `jj-remote-aspen` helper as the first client entrypoint.

**Rationale:** JJ-native sync must carry JJ object graphs, change-id state, and bookmark semantics. Reusing the Git remote-helper path would reintroduce the Git translation problem the change is meant to remove.

**Alternative:** Tunnel JJ through Git remote-helper commands. Rejected because it cannot represent JJ-native state cleanly and would tempt silent fallback behavior.

### 5. First cut sync scope is objects + change IDs + bookmarks

**Choice:** Initial server-side scope covers JJ objects, repo reachability, change-id lookup, and bookmark synchronization. Full remote operation-log/workspace semantics stay out of scope for now.

**Rationale:** This gets first-class JJ repository exchange working without taking on every local JJ concept at once.

**Alternative:** Model JJ operation logs and working-copy state on day one. Rejected because it expands the state model, retention story, and compatibility surface too much for the first change.

### 6. JJ pushes use staged visibility and atomic publish

**Choice:** Streamed JJ push data lands in a repo-scoped staging area first. Forge only updates visible JJ repo state after all chunks validate, all referenced objects are present, and one final consensus-backed publish step commits bookmark and change-id index updates.

**Rationale:** This gives the retry guarantee the spec requires. A failed stream leaves either fully unpublished staged data or a fully committed repo update, never a half-visible mix.

**Alternative:** Make pushed objects visible as they arrive. Rejected because partial streams would leak inconsistent state and make retries ambiguous.

### 7. Protocol-capable plugins run with host-enforced permissions and limits

**Choice:** The JJ plugin declares protocol identifiers plus explicit permissions for repo metadata reads/writes, JJ namespace access, and blob reads/writes. The host enforces repo-scoped KV/blob access, session timeout, maximum in-flight bytes, maximum chunk size, and maximum concurrent sessions.

**Rationale:** Protocol sessions are longer-lived than unary plugin calls, so the host must own both authorization and resource limits instead of trusting plugin code.

**Alternative:** Let plugins self-police protocol session access and sizing. Rejected because a buggy or malicious plugin could overrun bounds or escape repo isolation.

### 8. Dual-backend repos are co-located but separately managed in the first cut

**Choice:** A single repo record may enable both Git and JJ, but the first implementation treats them as co-located backends with separate histories, ref namespaces, and sync paths. No automatic Git<->JJ projection is attempted in this change.

**Rationale:** This satisfies backend coexistence without forcing semantic translation between two different VCS models.

**Alternative:** Promise mixed collaborative history between Git and JJ immediately. Rejected because it would require translation, conflict semantics, and browsing rules that need a separate design.

### 9. Capability discovery is repo-scoped and carries routing info

**Choice:** Forge exposes one repo capability surface that returns enabled backends plus per-backend transport identifiers before a client starts sync. JJ routing information is returned only for target nodes that currently advertise the active JJ plugin; nodes without that plugin return JJ as unavailable for that repo/node pair.

**Rationale:** Clients need one discovery step that answers both "does this repo support JJ?" and "how do I route the next protocol session?". Putting that in repo metadata avoids split-brain discovery between repo lookup and plugin routing while still respecting per-node plugin availability.

**Alternative:** Make clients discover routing entirely from node/plugin state after connecting. Rejected because repo support is repo-scoped, not just node-scoped.

### 10. Final publish uses optimistic conflict checks

**Choice:** The final publish step checks expected bookmark heads and change-id heads before committing JJ-visible state. If another publish has already advanced one of those heads, Forge rejects the publish with JJ-native conflict information instead of overwriting newer state.

**Rationale:** Atomic publish prevents partial visibility, but conflict checks are still needed so concurrent JJ pushes do not silently clobber bookmarks or change-id mappings.

**Alternative:** Last-writer-wins for bookmark and change-id updates. Rejected because it hides races from JJ clients and makes rewrite/push outcomes nondeterministic.

### 11. Repo authorization stays in native Forge admission

**Choice:** JJ-native protocol sessions pass through the same native Forge authentication and repo-ACL admission path used by other Forge operations before plugin-owned repo logic runs.

**Rationale:** Plugin sandbox permissions control what the plugin may touch inside the host. They are not a substitute for deciding whether a client may read or write a repo. Keeping client auth and ACL checks in native admission preserves one security boundary for Git and JJ alike.

**Alternative:** Let the JJ plugin evaluate repo ACLs from session metadata. Rejected because it duplicates security-critical logic inside plugin code and risks bypassing existing Forge policy.

### 12. JJ routing identifier comes from plugin registration

**Choice:** The JJ routing identifier is a manifest-declared transport identifier stored in plugin registration metadata and surfaced verbatim in capability discovery for capable nodes. Reloads/upgrades may continue to advertise the same identifier only when the declared identifier is unchanged.

**Rationale:** A registration-backed identifier gives one source of truth for both discovery output and QUIC session admission. It also makes stability verifiable across reloads and node restarts.

**Alternative:** Generate a fresh routing identifier at plugin load time. Rejected because clients could not rely on discovery stability and restarts would invalidate cached routing data unnecessarily.

### 13. JJ transport version is advertised beside the transport identifier

**Choice:** The JJ plugin registration records both a stable transport identifier and an explicit transport-version value. Capability discovery returns both. Session admission checks version compatibility before any JJ object exchange begins.

**Rationale:** Version checks are part of the wire contract, not an implementation detail. Clients need to know both where to connect and whether that endpoint speaks a compatible JJ protocol revision.

**Alternative:** Infer compatibility from the transport identifier alone. Rejected because it overloads routing identity with protocol evolution and makes upgrades ambiguous.

### 13a. Node activation state is separate from plugin registration

**Choice:** Plugin registration metadata says a JJ backend can run on a node; node activation state says it is currently serving sessions on that node. The source of truth for activation is the local plugin runtime registry after successful load. Each node publishes that active set into its node capability advertisement only after the JJ plugin is ready to admit sessions, and withdraws the advertisement before reload rollback or disablement makes the session handler unavailable.

**Rationale:** Registration and activation solve different problems. Discovery needs live per-node availability, not just installed metadata.

**Alternative:** Treat registration as activation. Rejected because an installed but unloaded or failed plugin would still be advertised as runnable.

### 14. Repo delete tombstones JJ metadata but not shared blobs

**Choice:** Deleting a repo removes or tombstones repo-scoped JJ bookmark namespaces, change-id indexes, and discovery metadata immediately, while leaving shared blob content to the existing retention/GC path.

**Rationale:** Clients must stop seeing the deleted repo right away, but aggressively deleting content-addressed blobs during repo deletion would couple repo semantics to global blob reachability and GC.

**Alternative:** Eagerly delete all JJ blobs on repo removal. Rejected because blobs may still be shared or referenced elsewhere and GC already owns physical reclamation.

### 15. Incremental sync uses explicit object and bookmark probes

**Choice:** JJ-native fetch and push begin with explicit object/bookmark/change-head probes against Forge indexes so each side can identify missing JJ objects and changed bookmark/change-index state before streaming bulk data.

**Rationale:** The specs require incremental fetch, incremental push, and bookmark-only sync. A probe-first contract makes those behaviors deterministic and testable instead of relying on implicit cache assumptions.

**Alternative:** Always stream the full reachable JJ graph and let the receiver deduplicate locally. Rejected because it violates the intended incremental contract and wastes bandwidth.

### 16. Staged data is quota-bound and expires

**Choice:** Repo-scoped staged JJ data is keyed by session, counted against host-enforced quota, and deleted on successful publish, timeout, explicit rejection, or cleanup sweep after session abandonment.

**Rationale:** Bounded network stages are not enough if unpublished staging can accumulate forever. The host needs an explicit lifetime and quota policy for staged objects.

**Alternative:** Keep failed staged data indefinitely for manual recovery. Rejected because it creates unbounded storage growth and blurs retry semantics.

### 16a. Repo deletion aborts sessions before publish can commit

**Choice:** Repo deletion and backend disablement first mark the repo unavailable in native metadata and withdraw JJ discovery for that repo/node pair. New JJ sessions are rejected immediately after that transition. In-flight JJ sessions check repo availability before each state transition, and final publish performs a last repo-generation/tombstone check before any bookmark or change-id write. If the repo was deleted or JJ was disabled after staging began, the session fails, visible JJ state stays unchanged, and staged entries under `forge:jj:stage:{repo_id}:{session_id}:*` are tombstoned for cleanup.

**Rationale:** Delete semantics and failed-push semantics only line up if deletion wins over in-flight publish.

**Alternative:** Let in-flight sessions finish after delete if they already staged data. Rejected because deleted repos would still accept backend-visible state changes after deletion.

## Risks / Trade-offs

- **[JJ upstream transport expectations may shift]** -> Keep the server request family narrow and isolate JJ client adaptation behind one helper layer.
- **[Dual-backend repos can confuse users]** -> Make backend support explicit in repo metadata and reject unsupported backend access without fallback.
- **[Large JJ graphs can stress plugin/runtime memory]** -> Use chunked bounded streams, repo-scoped limits, and blob-backed staging instead of whole-graph buffering.
- **[Plugin protocol routing may need runtime work]** -> Land the plugin/runtime routing pieces before the JJ plugin itself and cover them with focused transport tests.
- **[Garbage collection for rewritten JJ commits is tricky]** -> Keep retention conservative at first and defer aggressive pruning until JJ reachability rules are proven.

## Verification Strategy

- `specs/forge/spec.md` -> integration tests for repo create/list/delete with backend manifests, existing-repo default-to-`git` behavior when the manifest is absent, repository list responses that report enabled backends, node-specific capability-discovery responses for Git-only, JJ-only, and dual-backend repos, deleted-repo checks proving JJ namespaces/indexes/discovery metadata are no longer reachable, and checks that shared blobs remain on the normal retention/GC path.
- `specs/jj-native-forge/spec.md` -> integration and regression tests for native JJ object storage, cross-node JJ blob fetch on cache miss, change-id resolution after rewrite, malformed-payload rejection, concurrent final-publish conflict rejection, bookmark create/move/delete, namespace isolation from Git refs, and repo-delete/session-abort behavior that blocks final publish after delete.
- `specs/jj-remote-aspen/spec.md` -> end-to-end tests for native clone/fetch/push, incremental sync, change-id resolution, authorized/unauthorized JJ access, stable routing-identifier discovery on capable nodes, compatible/incompatible transport-version handling, node-specific plugin-unavailable discovery, activation-state advertisement changes during load/unload, explicit no-fallback behavior to `git-remote-aspen`, and bounded large-transfer failure handling.
- `specs/plugins/spec.md` -> runtime tests for protocol identifier registration, collision rejection, stable routing-identifier binding, protocol-session dispatch to the right plugin, permission/resource-limit enforcement during a session, and cleanup after limit-triggered termination.
- negative-path coverage -> named tests for Git access to JJ-only repos, JJ access to Git-only repos, and JJ access to JJ-enabled repos on nodes where the JJ plugin is inactive.
- design-only invariants -> targeted tests for staged publish behavior proving failed streams leave repo-visible JJ state unchanged, successful publish releases staged quota, and abandoned/limit-terminated sessions expire and clean up staged data.

## Migration Plan

1. Land plugin/runtime support for protocol-capable WASM handlers and JJ request families.
2. Add Forge repo metadata for backend advertisement, defaulting existing repos to `git` only.
3. Ship the JJ-native Forge plugin behind an explicit feature/config gate.
4. Allow new repos to opt into JJ support and existing repos to enable JJ deliberately.
5. Add JJ client tooling and integration tests before advertising the feature broadly.
6. Operational rollback for the first cut means stopping rollout on additional repos/nodes and hiding JJ routing on nodes where the plugin is disabled; this change does not define removal or garbage collection of already-written JJ state.

## Deferred Follow-up

- Additional JJ metadata indexes for server-side browsing/debugging are out of scope for this change. Revisit only in a follow-up change if a later spec requires them.
