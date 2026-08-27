## Phase 1: Dependency and pure admission

- [ ] [serial] [blocked:executable-extent-project] Record the accepted project contract, immutable source revision, license, Octet gate, conformance suite, and claim boundaries. r[molten.world_extents.dependency]
- [ ] [serial] [blocked:mantle-extent-bundle] Record the reviewed Mantle producer bundle schema, fixtures, and exact source identity. r[molten.world_extents.profile]
- [ ] [depends:introduce-world-commit-core] Record baseline artifact registry, runtime admission, content-store, and world code-root tests. r[molten.world_extents.verification]
- [ ] [serial] Define nominal semantic-code, artifact, extent-manifest, extent, page-profile, mapping, runtime-cohort, admission, and diagnostic DTOs. r[molten.world_extents.identity_domains] r[molten.world_extents.profile]
- [ ] [depends:world-extent-dtos] Implement pure manifest, layout, closure, target, ABI, page, permission, W^X, bound, and fallback validation by adapting the shared core. r[molten.world_extents.admission] r[molten.world_extents.wx]

## Phase 2: Shell and world integration

- [ ] [depends:world-extent-core] Add narrow bundle-read, capability-root, materialization, remeasurement, sealing, mapping, protection, read-back, unmap, and runtime-activation ports. r[molten.world_extents.materialization]
- [ ] [depends:world-extent-ports] Add the optional world code-root profile that binds exact artifact, extent manifest, producer, runtime cohort, and policy identities. r[molten.world_extents.profile]
- [ ] [depends:world-extent-ports] Implement capability-relative materialization and verify-by-handle without ambient path reopen. r[molten.world_extents.materialization]
- [ ] [depends:world-extent-materialization] Implement narrow W^X mapping and activation adapters with fresh read-back and current authority rechecks. r[molten.world_extents.wx] r[molten.world_extents.activation]
- [ ] [parallel] Add detached extent admission, mapping, activation, and unmap receipts without build, authority, or release overclaims. r[molten.world_extents.receipts]

## Phase 3: Verification and documentation

- [ ] [parallel] Add positive exact bundle, stable remeasurement, shared read-only mapping, executable-read-only activation, unmap, and explicit ordinary-artifact profile fixtures. r[molten.world_extents.verification]
- [ ] [parallel] Add negative misalignment, overlap, truncation, digest mismatch, source substitution, target mismatch, unsupported format, ABI mismatch, page mismatch, partial closure, writable-executable request, stale producer, missing authority, silent fallback, path reopen, and mapping-as-sandbox fixtures. r[molten.world_extents.verification]
- [ ] [serial] Document identity domains, producer and consumer ownership, W^X states, capability-relative mapping, fallback, retention, and non-claims. r[molten.world_extents.receipts]
- [ ] [depends:world-extent-verification] Run focused tests, shared conformance vectors, unsafe-code audit, Octet, Clippy with warnings denied, Cairn validation and gates, lifecycle checks, and relevant Nix checks. r[molten.world_extents.verification]
