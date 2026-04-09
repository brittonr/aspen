# Federation Orchestration Audit

Compared the current Rust dogfood flow with the deprecated shell implementation in
`scripts/deprecated/dogfood-federation.sh`, focusing on `do_federate()`,
`do_sync()`, and `do_build()`.

## Missing shell-parity steps identified

### `do_federate()`

The shell flow explicitly federated Alice's Forge repo before Bob tried to mirror
it:

- call `federation federate <repo_id> --mode public` on Alice
- tolerate the repo already being federated

### `do_sync()`

The shell flow prepared Bob to discover Alice's federated repo:

- derive Alice's federation peer identity from Alice's ticket / health state
- select a direct address hint for Alice (IPv4 preferred)
- invoke `federation sync --peer <alice_node_id> [--addr <alice_addr>]` on Bob
- inspect Bob's `_fed:mirror:` metadata after sync

### `do_build()`

The shell flow did more than wait for CI on Bob:

- ensure Bob has a local Forge repo for CI (`aspen-mirror`)
- register `ci watch` on Bob's mirror repo
- federated-clone Alice's repo through Bob using a `fed:` URL
- push the cloned content into Bob's Forge repo
- wait for the pipeline on Bob's mirror repo rather than hardcoding Alice's repo name

## Rust mapping chosen for the port

- `crates/aspen-dogfood/src/federation.rs`
  - `prepare_build(...)` owns the Alice→Bob handoff
  - `federate_repo(...)` ports `do_federate()`
  - `sync_repo(...)` ports the Bob-side sync step
  - `clone_repo(...)` + `push_clone(...)` port the federated clone / Bob mirror push
- `crates/aspen-dogfood/src/main.rs`
  - federation `cmd_build()` now calls `federation::prepare_build(...)`
  - CI waits on Bob's mirrored repo name returned by `prepare_build(...)`
- `crates/aspen-dogfood/src/forge.rs`
  - shared repo lookup and PATH helper reuse for federation git subprocesses
- `crates/aspen-dogfood/src/ci.rs`
  - shared repo lookup for resolving Bob's mirrored repo ID before CI waits / triggers
