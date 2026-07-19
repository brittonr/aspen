## Phase 1: Baseline and exact mapping

- [x] [serial] I1 Record the current core/shell verification baseline, immutable standalone source, completion contract, approach registry, authority boundaries, and false-completion cases. r[molten.artifact_auth_shell.exact_verification] r[molten.artifact_auth_shell.authority]
- [x] [serial] I2 Expose a pure deterministic core mapping for the exact signer-specific standalone statement without adding effects or accepting legacy verification as standalone proof. r[molten.artifact_auth_shell.exact_verification]

## Phase 2: Product shell pilot

- [x] [depends:molten.artifact_auth_shell.exact_verification] I3 Add purpose-bounded signing of exact standalone canonical bytes through the capability-file adapter and verification through the pinned `artifact-auth-ed25519` package without exposing private material. r[molten.artifact_auth_shell.exact_verification] r[molten.artifact_auth_shell.authority]
- [x] [depends:molten.artifact_auth_shell.exact_verification] I4 Emit bounded public statement/key/signature identities, lowercase signature hex, cryptographic failure class, and the existing dual-run compatibility result. r[molten.artifact_auth_shell.evidence]
- [x] [depends:molten.artifact_auth_shell.evidence] I5 Document operator integration, immediate rollback, retained product authority, and why this pilot does not admit standalone runtime authority. r[molten.artifact_auth_shell.authority]

## Phase 3: Verification

- [x] [parallel] V1 Add real positive and negative shell tests for exact signature parity, tampered statement/signature, wrong preimage/key, malformed lengths, carrier identity drift, revoked/unknown currentness, legacy-boolean reuse, secret exclusion, and authority non-promotion. r[molten.artifact_auth_shell.exact_verification] r[molten.artifact_auth_shell.evidence] r[molten.artifact_auth_shell.authority]
- [x] [serial] V2 Run focused core/shell tests, strict Clippy, Octet, exact-source/unit2nix checks, full Cargo/Nix validation, Cairn gates, accepted-spec sync, and archive with bounded evidence. r[molten.artifact_auth_shell.evidence] r[molten.artifact_auth_shell.authority]
