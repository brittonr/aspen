## Why

Issue #46: node identity and production key files were written in place with create plus truncate, a flush instead of a sync, and no rename or directory synchronization. A crash during first-boot generation or key rotation could leave a truncated key or destroy the current one, and the owner-only mode applied only when the file was first created.

## What Changes

- `node_state::Scope::write_restricted` replaces a secret through a staged publication: the payload is written and synchronized under a stage name in the same directory, renamed over the destination, and the directory is synchronized. Rotation and revocation markers use it.
- A new `Scope::write_new_restricted` creates a secret the same way but links the stage to the destination without replacement, so an existing key is never overwritten. First-boot generation of production keys and the node transport identity use it.
- Every publication leaves the destination owner-restricted, including when it replaces a looser existing file. Stages left by an interrupted publication of the same leaf are removed before the next one and are never read as keys.
- A failure after the destination changed is reported as an uncertain external outcome instead of a clean failure.

## Impact

- **Files**: `crates/molten-node-host/src/node/state/{publication,namespace,filesystem,authority}.rs`, `src/fabric_crypto_identity/file/parts/adapter/p001/body.rs`, `src/node/parts/identity/{p003,p004}/body.rs`, `docs/node-state-filesystem-authority.md`.
- **Testing**: node-host publication tests for first creation, refusal to overwrite, whole replacement with restricted mode, stale-stage removal and symlink or non-regular destinations; the existing crypto-identity, node-identity and reliability-authority suites that write secrets.
- **Owner / consumer**: `molten-node-host` owns capability-rooted node state; the crypto identity adapter, node identity startup and the reliability authority store consume it.
