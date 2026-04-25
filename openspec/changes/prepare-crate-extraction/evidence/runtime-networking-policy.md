# Runtime networking policy evidence

## Scope

Covers V2 runtime-networking guardrail for `prepare-crate-extraction`: Aspen runtime/shipped adapters remain iroh-only, and the reusable storage/type/facade defaults do not introduce arbitrary non-iroh runtime transport.

## Policy source

`docs/crate-extraction/policy.ncl` lists concrete transport crates as:

- `iroh`
- `iroh-base`
- `irpc`
- `aspen-transport`
- `aspen-raft-network`

The first Redb Raft KV split keeps concrete transport ownership in `aspen-raft-network` and the Aspen compatibility shell in `aspen-raft`; storage/type/facade defaults must not construct endpoints.

## Source audit

- Command transcript: `openspec/changes/prepare-crate-extraction/evidence/runtime-networking-policy-source-audit.txt`
- Result: `crates/aspen-raft-kv-types/src`, `crates/aspen-raft-kv/src`, and `crates/aspen-redb-storage/src` contain only documentation mentions of iroh/adapter terms and no concrete endpoint construction or non-iroh transport APIs.
- Result: `crates/aspen-raft-network/Cargo.toml` and `crates/aspen-raft/Cargo.toml` remain the explicit runtime/adapter locations for `iroh`, `iroh-base`, `irpc`, `aspen-transport`, `openraft`, and `tokio`.
- Result: the non-iroh runtime transport audit over `crates/aspen-raft-kv/src`, `crates/aspen-raft-kv-types/src`, `crates/aspen-redb-storage/src`, `crates/aspen-raft-network/src`, and `crates/aspen-raft/src/network` produced no `TcpListener`, `TcpStream`, `UdpSocket`, `axum`, `hyper`, `tonic`, `quinn`, `reqwest`, or `warp` matches.

## Conclusion

The current extraction slice preserves Aspen's shipped runtime networking policy: reusable defaults are transport-neutral, the explicit adapter surface remains iroh/IRPC, and no arbitrary non-iroh runtime transport was introduced by the reusable facade.
