## 1. Protocol and metadata foundation

- [x] 1.1 Extend Forge repo metadata, repository list responses, and client-facing capability discovery so a repo can declare `git`, `jj`, or both backends and return node-specific routing identifiers for active backends.
- [ ] 1.2 Update repository create/delete flows so JJ-enabled repos allocate JJ namespaces on create and remove/tombstone JJ namespaces, change-id indexes, and discovery metadata on delete while leaving shared blobs on the normal retention/GC path.
- [x] 1.3 Default or backfill pre-existing repositories with no backend manifest to `git`-only behavior.
- [x] 1.4 Add JJ-native request/response families, transport identifiers, and explicit transport-version advertisement/compatibility checks for clone, fetch, push, bookmark sync, and change-id lookup.
- [x] 1.5 Extend the plugin/runtime manifest and routing path so a WASM plugin can claim a bounded repo protocol session, not only unary requests.
- [x] 1.6 Reject protocol-identifier collisions during plugin registration/activation and surface a deterministic error.
- [x] 1.7 Persist the manifest-declared JJ routing identifier in plugin registration metadata, surface it in capability discovery, and keep it stable across reload/upgrade when unchanged.
- [x] 1.8 Publish and withdraw node-local JJ activation state so discovery only advertises nodes with an active JJ plugin.
- [ ] 1.9 Implement host/runtime enforcement of declared KV/blob/protocol permissions, session limits, and cleanup on denied or terminated protocol sessions.

## 2. JJ-native Forge storage

- [x] 2.1 Define the native JJ object model and blob encoding used by Forge for commits, trees, files, conflicts, and related metadata.
- [x] 2.2 Implement repo-scoped JJ object persistence and reachability lookup on top of BLAKE3-addressed blobs.
- [ ] 2.3 Implement cross-node JJ blob fetch through Aspen's blob distribution path so peers can satisfy missing-object reads.
- [x] 2.4 Implement Raft-backed JJ change-id indexes and JJ bookmark namespaces, including create/move/delete semantics.
- [x] 2.5 Implement staged JJ push publication so partial or failed streams do not make repo-visible state inconsistent.
- [x] 2.6 Validate incoming JJ payloads and reject malformed or inconsistent object graphs before final publish.
- [x] 2.7 Add optimistic final-publish conflict checks for stale bookmark heads and stale change-id heads.
- [x] 2.8 Add staged-data quota, expiry, and cleanup behavior for successful publish, timeout, rejection, and abandoned-session paths.
- [x] 2.9 Abort or reject in-flight JJ sessions when a repo is deleted or JJ support is disabled, and block final publish after that transition.

## 3. JJ Forge plugin and client path

- [ ] 3.1 Implement the JJ-native Forge WASM plugin with declared permissions, storage prefixes, request handlers, and protocol-session handlers.
- [ ] 3.2 After the 1.x discovery/routing foundation, 2.x storage/publish foundations, and 3.1 plugin session support land, implement standalone `jj-remote-aspen` session setup, capability discovery, version checks, and QUIC admission.
- [ ] 3.3 Implement standalone `jj-remote-aspen` clone/fetch and the probe-first incremental sync path for missing JJ objects.
- [ ] 3.4 Implement standalone `jj-remote-aspen` push and bookmark mutation flows, including explicit object/bookmark/change-head probes and conflict reporting.
- [ ] 3.5 Implement standalone `jj-remote-aspen` change-id resolution over the JJ-native path.
- [x] 3.6 Reject unsupported JJ access explicitly for Git-only repos and reject silent fallback to Git transport.
- [ ] 3.7 Return `capability-unavailable` for JJ-enabled repos when the selected target node does not currently have the JJ plugin active, and omit JJ routing identifiers for that node from discovery.
- [ ] 3.8 Enforce existing Forge authentication and repo read/write authorization rules on JJ-native clone, fetch, push, bookmark sync, and change-id resolution.

## 4. Dual-backend behavior and rollout

- [x] 4.1 Support JJ-only and dual-backend repos without collisions between Git refs and JJ bookmark/change namespaces.
- [ ] 4.2 Add feature/config wiring and plugin-loading paths so JJ support can be enabled deliberately per deployment and per repo.
- [x] 4.3 Document operator and developer workflows for creating JJ-enabled repos and connecting JJ clients.

## 5. Verification

- [ ] 5.1 Add integration tests for Forge repo create/list/delete flows with backend manifests, repository list responses that report enabled backends, pre-existing repos that default to `git` when the manifest is absent, node-specific capability-discovery responses for Git-only, JJ-only, and dual-backend repos, positive routing-identifier presence on active JJ nodes, explicit post-delete rejection of both Git and JJ operations, post-delete unreachability of JJ discovery metadata, and shared-blob retention on the normal GC path.
- [ ] 5.2 Add integration tests for authorized JJ-native clone, incremental fetch, probe-first incremental push, bookmark movement, change-id resolution, cross-node JJ blob fetch, compatible/incompatible transport-version handling, and explicit no-fallback behavior to `git-remote-aspen` against a JJ-enabled Forge repo.
- [ ] 5.3 Add regression tests for change-id preservation, rewrite/update behavior, malformed-payload rejection, staged-publish no-partial-visibility, concurrent final-publish conflict rejection, repo-delete session abort/final-publish blocking, and dual-backend namespace isolation.
- [ ] 5.4 Add negative tests proving JJ-to-Git-only and Git-to-JJ-only access fail with capability errors, JJ-enabled repos on plugin-inactive target nodes return `capability-unavailable`, discovery omits JJ routing identifiers for those nodes, unauthorized JJ writes are rejected, and no path falls back silently.
- [ ] 5.5 Add runtime tests proving plugin registration records protocol metadata, protocol-identifier collisions are rejected, discovery advertises the stable JJ routing identifier from active plugin registration, activation-state changes are reflected in discovery, and JJ-native QUIC session admission accepts that same advertised identifier.
- [ ] 5.6 Add runtime tests proving protocol sessions enforce declared permissions/resource limits and clean up per-session resources after timeout or limit-triggered termination.
- [ ] 5.7 Add transport/runtime tests proving large JJ transfers stay bounded, staged data is released after successful publish, staged data expires/cleans up on failed or abandoned sessions, and failed streams return retryable errors.
