## Phase 1: Correctness

- [x] Fix `DogfoodState::new_federation()` — now writes `is_federation: true` (`state.rs:77`).
- [x] Give alice and bob distinct cookies — `alice_cookie()` / `bob_cookie()` in `RunConfig`, passed through `spawn_node()` per-node (`main.rs`, `cluster.rs`, `node.rs`).
- [x] Restore federation env vars — extracted `federation_env()` pure function in `cluster.rs`, called from `start_federation()`. Covers `ASPEN_FEDERATION_ENABLED`, `_CLUSTER_KEY`, `_CLUSTER_NAME`, `_ENABLE_DHT_DISCOVERY`, `_ENABLE_GOSSIP`, and `ASPEN_CI_FEDERATION_CI_ENABLED` on bob.
- [x] Restore `CiWatchRepo` registration before `git push` — new `forge::watch_repo()` called from `cmd_push()` (`forge.rs`, `main.rs`).
- [x] Remove dead `--node-count` flag — removed from CLI, `RunConfig`, and all call sites.
- [x] Align `full` / `full-loop` semantics — `full` is the complete pipeline (`start -> push -> build -> deploy -> verify -> stop`), and `full-loop` is `build -> deploy -> verify` with the cluster already running. Updated `docs/deploy.md` and `AGENTS.md`.
- [x] Fix `cmd_build` ticket selection — uses `state.bob_ticket()` instead of raw index lookup via `tickets().get(1)`.

## Phase 2: Verification

- [x] `cargo test -p aspen-dogfood`: 26 passed. Tests cover `cluster::tests` (federation env construction, ticket preview), `state::tests` (federation flag, bob_ticket), and `tests` (cookie distinctness, date content, state file path, `--node-count` rejection).
- [x] `cargo clippy -p aspen-dogfood -- --deny warnings`: clean.
- [x] NixOS VM test `nix/tests/dogfood-binary-smoke.nix` added and executed via `nix build .#checks.x86_64-linux.dogfood-binary-smoke-test --option sandbox false` — exercises `aspen-dogfood start`, `status` (asserts `status=healthy` and `node_id=1`), `push` (asserts ordered `repo created -> CI watch registered -> git push`), and `stop` (asserts the cluster dir is removed). Uses forge-enabled node (`ci-aspen-node-snix-build`). Wired into `flake.nix` as `dogfood-binary-smoke-test`.

## Follow-up

Full federation orchestration parity (`federate` / `sync` / mirror creation) is tracked separately in `openspec/changes/active/2026-04-08-port-dogfood-federation-orchestration/`.
