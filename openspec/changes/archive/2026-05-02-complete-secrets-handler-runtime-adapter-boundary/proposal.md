## Why

`aspen-secrets-handler --no-default-features` still pulled the Aspen runtime context graph through `aspen-rpc-core/runtime-context` and named KV contracts through `aspen-core`. That made the handler's portable executor/test surface depend on Redb/Raft/runtime storage even after `aspen-secrets` itself moved to lightweight KV traits.

## What Changes

- Gate only the handler factory/runtime-context adapter behind `aspen-secrets-handler/runtime-adapter`.
- Keep the default `SecretsServiceExecutor` and native PKI/Nix-cache dispatch surface on the reusable RPC executor plus lightweight KV trait/type crates.
- Preserve node/runtime compatibility by enabling the adapter feature from `aspen-rpc-handlers`' `secrets` bundle.

## Impact

- **Crates**: `aspen-secrets-handler`, `aspen-rpc-handlers`.
- **APIs**: `SecretsHandlerFactory` and re-exported runtime-context factory types require `runtime-adapter`; executor/service remain available by default.
- **Dependencies**: default handler builds avoid `aspen-core`, `aspen-core-shell`, `aspen-raft`, `aspen-redb-storage`, and `redb`.
- **Testing**: targeted handler checks/tests, no-default dependency grep, runtime-adapter check, aggregate RPC handler check, node secrets bundle check.
