# Service/Runtime Follow-up Selection

This evidence records broader service/runtime candidate decisions for `prepare-crate-extraction`. No new follow-up OpenSpec proposals are created by this task.

## Selected for immediate manifest stubs in this change

| Family | Decision | Rationale | Artifact |
| --- | --- | --- | --- |
| Foundational types/helpers | Create stub now. | Foundational leaks (`aspen-storage-types` Redb table definitions and `aspen-traits` transitive defaults) block many other extraction paths. | `docs/crate-extraction/foundational-types.md` |
| Auth and tickets | Create stub now. | Portable `aspen-auth-core` and `aspen-hooks-ticket` are already split from runtime shells and need a documented contract. | `docs/crate-extraction/auth-ticket.md` |
| Protocol/wire | Create stub now. | `aspen-client-api` and protocol crates define compatibility-sensitive wire schemas and must stay independent from handlers/runtime transports. | `docs/crate-extraction/protocol-wire.md` |

## Deferred service/runtime candidates

| Family | Decision | Rationale | Inventory status |
| --- | --- | --- | --- |
| Iroh transport/RPC | Defer to separate change. | Needs deeper split between generic iroh RPC helpers, client APIs, and handler registry wiring. | `manifest not yet created` in `docs/crate-extraction.md` |
| Coordination | Defer to separate change. | Should depend on reusable KV traits/types and injected time; storage/facade boundaries must settle first. | `manifest not yet created` |
| Blob/castore/cache | Defer to separate change. | Needs separation between Aspen client integration and standalone storage/cache APIs. | `manifest not yet created` |
| Commit DAG / branches | Defer to separate change. | Several crates still depend directly on `aspen-raft` where trait-based KV may be enough. | `manifest not yet created` |
| Jobs and CI core | Defer to separate change. | Scheduler/schema can be reusable, but worker runtime/executor integration is larger than this first Redb Raft KV slice. | `manifest not yet created` |
| Trust/crypto/secrets | Defer to separate change. | Pure crypto may be extractable, but secrets runtime/transport integration needs independent policy. | `manifest not yet created` |
| Config/plugin | Defer to separate change. | `aspen-nickel` and `aspen-plugin-api` need standalone examples and feature minima after core boundaries settle. | `manifest not yet created` |
| Testing harness | Defer to separate change. | Generic harness helpers must be separated from Aspen cluster boot helpers. | `manifest not yet created` |
| Binary shells | Defer to separate audit tasks only. | CLI/TUI/node/web/bridge/gateway/dogfood stay final consumers; reusable behavior should migrate to libraries as discovered. | `manifest not yet created` |

## Guardrails

- Deferred means not implemented in this change, not rejected.
- Future follow-up proposals require separate user or planning action.
- Inventory rows in `docs/crate-extraction.md` keep owner status, readiness state, and next action visible so deferred work is not lost.
