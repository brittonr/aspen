## Phase 1: Dependency and pure admission

- [x] [serial] Record private Radicle source `rad://z37R1bP1kHcELs89RNbQRaqbCVKxB`, archived revision `025d9636f0161777710dac37b3c210ca0ad9483f`, license, Octet gate, conformance suite, and claim boundaries. r[molten.world_extents.dependency]
- [x] [serial] Record Mantle producer `rad://z3DJe8tEdQuXpzTkfqCYQq6ZUqqkb` at `2c636b1b25353a1b0befa5af48dc68615cd686dd`, its bundle schema, fixtures, and exact source identity. r[molten.world_extents.profile]
- [x] [depends:introduce-world-commit-core] Record baseline artifact registry, runtime admission, content-store, and world code-root tests. r[molten.world_extents.verification]
- [x] [serial] Define nominal semantic-code, artifact, extent-manifest, extent, page-profile, mapping, runtime-cohort, admission, and diagnostic DTOs. r[molten.world_extents.identity_domains] r[molten.world_extents.profile]
- [x] [depends:world-extent-dtos] Implement pure manifest, layout, closure, target, ABI, page, permission, W^X, bound, and fallback validation by adapting the shared core. r[molten.world_extents.admission] r[molten.world_extents.wx]

## Phase 2: Shell and world integration

- [x] [depends:world-extent-core] Add narrow bundle-read, capability-root, remeasurement, mapping, unmap, and runtime-admission ports. r[molten.world_extents.materialization]
- [x] [depends:world-extent-ports] Add the optional world code-root profile that binds exact artifact, extent manifest, producer, runtime cohort, and policy identities. r[molten.world_extents.profile]
- [x] [depends:world-extent-ports] Implement capability-relative materialization and verify-by-handle without ambient path reopen. r[molten.world_extents.materialization]
- [x] [depends:world-extent-materialization] Implement narrow W^X mapping with fresh read-back, current authority rechecks, and explicit unmap. r[molten.world_extents.wx] r[molten.world_extents.activation]
- [x] [parallel] Add detached extent admission, mapping, activation-denial, and unmap receipts without build, authority, or release overclaims. r[molten.world_extents.receipts]

## Phase 3: Verification and documentation

- [x] [parallel] Add positive exact bundle, stable remeasurement, executable-read-only mapping, unmap, and explicit ordinary-artifact profile fixtures. r[molten.world_extents.verification]
- [x] [parallel] Add negative layout, truncation, digest, substitution, target, format, ABI, page, closure, W^X, stale producer, authority, fallback, traversal, and overclaim fixtures. r[molten.world_extents.verification]
- [x] [serial] Document identity domains, producer and consumer ownership, W^X states, capability-relative mapping, fallback, retention, and non-claims. r[molten.world_extents.receipts]
- [x] [serial] Archive and publish the executable-extent compatibility change, replace draft pins with its final immutable revision, and regenerate the consumer receipt. r[molten.world_extents.dependency]
- [x] [depends:world-extent-verification] Run focused tests, shared conformance vectors, unsafe-code audit, Octet, Clippy with warnings denied, Cairn validation and gates, lifecycle checks, and relevant Nix checks. r[molten.world_extents.verification]
