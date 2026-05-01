# I11 trust/crypto/secrets surface inventory

Status: started after completing the jobs/CI core slice.

## Reusable default candidates

- `aspen-trust`: Shamir/GF256/HKDF/share-chain/envelope/reconfiguration helpers are the primary pure trust surface. Current dependency tree stays in crypto/serde/tokio-sync style libraries and does not pull Aspen Raft, Redb, handler registries, or node bootstrap shells.
- `aspen-secrets --no-default-features`: keeps the existing secrets library buildable with default features disabled; trust/SOPS/client transport functionality remains feature-gated.
- `aspen-crypto`: reusable hash/cookie/key lifecycle helpers exist, but the current default crate still carries Iroh/tokio filesystem lifecycle utilities. Treat transport-free helpers as candidates and Iroh identity/runtime helpers as boundary risks until I11/I12 split or fixture evidence narrows the reusable surface.

## Runtime/adapter boundary

- `aspen-secrets-handler` remains an RPC/service adapter and compatibility consumer.
- `aspen-secrets` SOPS/client/transport feature set remains outside the portable default boundary.
- concrete Iroh endpoint/identity generation, node bootstrap, Raft application, Redb storage, handler registries, and filesystem/SOPS IO remain runtime shells.

## Evidence captured

`i11-trust-crypto-secrets-inventory.txt` records successful checks and normal dependency trees for `aspen-trust`, `aspen-secrets --no-default-features`, and `aspen-crypto`.
