## Phase 1: Source identity

- [x] [serial] Bind the accepted artifact-auth publication receipt and exact Radicle HTTPS source. r[molten.artifact_auth_adoption.radicle_transport]
- [x] [serial] Cut Cargo, Nix, and release-policy sources to Radicle HTTPS, then regenerate Cargo/Nix locks and both unit2nix plans with owning tools without revision, content, or graph drift. r[molten.artifact_auth_adoption.radicle_agreement]

## Phase 2: Deterministic acceptance

- [x] [parallel] Run focused Molten core and shell artifact-auth tests before and after cutover without Rust implementation changes. r[molten.artifact_auth_adoption.radicle_behavior]
- [x] [serial] Prove GitHub fallback, mismatched RID/revision, duplicate or missing packages, stale policy, and stale build-plan identity are rejected. r[molten.artifact_auth_adoption.radicle_fallback]
- [x] [serial] Emit typed BLAKE3 cutover evidence, run focused Nix and Cairn checks, sync the accepted spec, and archive at the bounded claim boundary. r[molten.artifact_auth_adoption.radicle_evidence]
