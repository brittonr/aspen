## Why

The `aspen-dogfood` Rust binary replaced the deprecated shell scripts in the flake apps (`nix run .#dogfood-local`, `.#dogfood-federation`) but shipped with regressions that broke federation mode, skipped CI auto-trigger registration, and left a dead `--node-count` flag. A static audit against the deprecated scripts surfaced six bugs, and runtime verification surfaced one more client-shutdown bug in the dogfood RPC path.

## Scope

This change fixes the identified regressions and adds verification. It does **not** port the full federation orchestration pipeline (federate/sync/mirror creation steps from the old `dogfood-federation.sh`). That work is tracked separately in `openspec/changes/active/2026-04-08-port-dogfood-federation-orchestration/`.

## What Changed

### Bug fixes

1. **`DogfoodState::new_federation()` wrote `is_federation: false`** — `state.rs:77`. Every federation-aware branch (`cmd_build`, `cmd_status`) checked this flag, so federation mode silently ran as single-cluster mode. Fixed: now writes `true`.

2. **Federation clusters shared a single cookie** — `spawn_node()` derived the cookie from shared `RunConfig::cookie()`. Two clusters with the same cookie and node-id 1 on the same host risk cross-cluster confusion. Fixed: `RunConfig` now has `alice_cookie()` / `bob_cookie()`, and `spawn_node()` takes `cookie` as a parameter instead of deriving it internally.

3. **Federation env vars were missing** — the Rust path only set relay/docs/hooks/cache env vars. The deprecated `dogfood-federation.sh:170-174` also exported `ASPEN_FEDERATION_ENABLED`, `ASPEN_FEDERATION_CLUSTER_KEY`, `ASPEN_FEDERATION_CLUSTER_NAME`, disabled DHT/gossip, and set `ASPEN_CI_FEDERATION_CI_ENABLED` on bob. Fixed: extracted `federation_env()` pure function in `cluster.rs`, called from `start_federation()`.

4. **Local push skipped `CiWatchRepo`** — the old `dogfood-local.sh:509-511` registered a watch before `git push` so the auto-trigger path fires. The Rust binary only created the repo and pushed. Fixed: new `forge::watch_repo()` sends `CiWatchRepo` RPC before `git_push()`.

5. **`--node-count` was a dead flag** — parsed, stored, never used. The `add_learner()` / `change_membership()` helpers in `cluster.rs` were dead code. Fixed: removed the flag from CLI and `RunConfig`. The helpers remain (with `#[allow(dead_code)]`) for future multi-node support.

6. **`cmd_build` ticket selection used fragile index lookup** — `state.tickets().get(1).unwrap_or(...)` instead of the now-available `state.bob_ticket()`. Fixed.

7. **Dogfood RPC clients were dropped without `AspenClient::shutdown()`** — repeated `start` / `status` / `push` / `build` / `deploy` / `verify` commands created short-lived client endpoints and let them drop, which emitted `Endpoint dropped without calling Endpoint::close` errors during runtime smoke. Fixed: `forge.rs`, `ci.rs`, and `deploy.rs` now explicitly call `shutdown()` before returning, and `cluster.rs::check_health()` now routes through `check_health_with_client()` so shutdown runs on both success and RPC-error return paths.

### Doc alignment

- `docs/deploy.md`: corrected `full-loop` description to match actual semantics (build -> deploy -> verify, cluster already running) and added `full` as the complete pipeline command.
- `AGENTS.md`: dogfood section now explicitly documents `full` as the complete pipeline and `full-loop` as `build -> deploy -> verify` with the cluster already running.

### VM test

- `nix/tests/dogfood-binary-smoke.nix`: NixOS VM test that exercises `aspen-dogfood start`, `status` (asserts `status=healthy` and `node_id=1`), `push` (asserts ordered `repo created -> CI watch registered -> git push` via `cmd_push`), and `stop` (asserts the cluster dir is removed). Uses forge-enabled node (`ci-aspen-node-snix-build`). Wired into `flake.nix` as `dogfood-binary-smoke-test`.

## Evidence

### `cargo test -p aspen-dogfood` (27 passed, 0 failed)

Verbatim output:

```
running 27 tests
test cluster::tests::federation_env_bob_has_ci_enabled ... ok
test cluster::tests::federation_env_alice_has_required_vars ... ok
test cluster::tests::federation_env_key_matches_input ... ok
test cluster::tests::federation_env_vm_ci_adds_executor ... ok
test cluster::tests::ticket_preview_truncates ... ok
test cluster::tests::ticket_preview_short ... ok
test error::tests::error_display_client_rpc ... ok
test error::tests::error_display_health_check ... ok
test error::tests::error_display_no_cluster ... ok
test cluster::tests::check_health_with_client_shutdowns_on_rpc_error ... ok
test error::tests::error_display_process_spawn ... ok
test error::tests::error_display_timeout ... ok
test state::tests::bob_ticket_federation ... ok
test state::tests::bob_ticket_single_falls_back ... ok
test state::tests::delete_state_nonexistent_is_ok ... ok
test state::tests::new_federation_is_federation ... ok
test state::tests::new_single_is_not_federation ... ok
test state::tests::node_pids_and_tickets ... ok
test state::tests::read_state_missing_returns_no_cluster ... ok
test state::tests::state_roundtrip_single ... ok
test state::tests::state_roundtrip_federation ... ok
test state::tests::read_state_corrupt_returns_deserialize_error ... ok
test state::tests::state_file_write_read_delete ... ok
test tests::state_file_path_uses_cluster_dir ... ok
test tests::cookie_contains_date ... ok
test tests::cookies_are_distinct_across_modes ... ok
test tests::node_count_flag_rejected ... ok

test result: ok. 27 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### `cargo clippy -p aspen-dogfood -- --deny warnings`

Clean (exit 0).

### NixOS VM test

`nix build .#checks.x86_64-linux.dogfood-binary-smoke-test --option sandbox false`

Verbatim excerpt:

```
vm-test-run-dogfood-binary-smoke> subtest: aspen-dogfood start creates a running cluster
vm-test-run-dogfood-binary-smoke> machine: State OK: 1 node, ticket present
vm-test-run-dogfood-binary-smoke> subtest: aspen-dogfood status reports reachable node
vm-test-run-dogfood-binary-smoke> machine: Status OK: 2026-04-08T04:42:38.624137Z  INFO aspen_dogfood: ✅ node: status=healthy, node_id=1, uptime=5s
vm-test-run-dogfood-binary-smoke> subtest: aspen-dogfood push exercises repo creation and CiWatchRepo
vm-test-run-dogfood-binary-smoke> machine: Push rc=0, log: 2026-04-08T04:42:38.660179Z  INFO aspen_dogfood: 📦 Pushing source to Forge...
vm-test-run-dogfood-binary-smoke> 2026-04-08T04:42:38.688102Z  INFO aspen_dogfood::forge:   repo created (id: 1bb1caf243377e6b)
vm-test-run-dogfood-binary-smoke> 2026-04-08T04:42:38.699446Z  INFO aspen_dogfood::forge:   CI watch registered for repo 1bb1caf243377e6b
vm-test-run-dogfood-binary-smoke> 2026-04-08T04:42:38.710632Z  INFO aspen_dogfood::forge:   git push aspen-dogfood main...
vm-test-run-dogfood-binary-smoke> 2026-04-08T04:42:41.906617Z  INFO aspen_dogfood: ✅ Source pushed to Forge
vm-test-run-dogfood-binary-smoke> machine: Push succeeded end-to-end
vm-test-run-dogfood-binary-smoke> subtest: aspen-dogfood stop cleans up
vm-test-run-dogfood-binary-smoke> machine: must succeed: test ! -e /tmp/dogfood-smoke
vm-test-run-dogfood-binary-smoke> machine: Cluster stopped and cleaned up
vm-test-run-dogfood-binary-smoke> test script finished in 23.39s
```

### Federation runtime smoke

Commands run:

```bash
nix run .#dogfood-federation -- --cluster-dir /tmp/aspen-dogfood-federation-proof start
nix run .#dogfood-federation -- --cluster-dir /tmp/aspen-dogfood-federation-proof status
python3 - <<'PY'
import json
with open('/tmp/aspen-dogfood-federation-proof/dogfood-state.json', 'r', encoding='utf-8') as fh:
    data = json.load(fh)
print({
    'is_federation': data['is_federation'],
    'labels': [node['label'] for node in data['nodes']],
    'node_count': len(data['nodes']),
})
PY
nix run .#dogfood-federation -- --cluster-dir /tmp/aspen-dogfood-federation-proof stop
```

Relevant runtime excerpt from the command logs:

```
2026-04-08T15:28:44.132532Z  WARN endpoint{id=cd3bf6448a}:RemoteStateActor{remote=d1f154bb19}: iroh::socket::remote_map::remote_state: Address Lookup failed: Service 'dns' error: no calls succeeded: [Failed to resolve TXT record...]
2026-04-08T15:28:44.231405Z  INFO aspen_dogfood::cluster:   node reachable (status=unhealthy)
2026-04-08T15:28:47.245783Z  INFO aspen_dogfood::cluster:   establishing federation trust...
2026-04-08T15:28:55.398164Z  INFO aspen_dogfood: ✅ Federation clusters running (alice + bob)
2026-04-08T15:29:01.772549Z  WARN endpoint{id=184e48a11f}:actor:QADv6{relay_url=https://usw1-1.relay.n0.iroh-canary.iroh.link./}:poll_send{dst=Ip([2a01:4ff:1f0:4b7b::1]:7842) src=None len=1200}: noq_udp: sendmsg error: Os { code: 101, kind: NetworkUnreachable, message: "Network is unreachable" }, ...
2026-04-08T15:29:04.847108Z  INFO aspen_dogfood: ✅ alice: status=healthy, node_id=1, uptime=24s
2026-04-08T15:29:07.924387Z  INFO aspen_dogfood: ✅ bob: status=healthy, node_id=1, uptime=27s
{'is_federation': True, 'labels': ['alice', 'bob'], 'node_count': 2}
2026-04-08T15:29:10.242707Z  INFO aspen_dogfood: ✅ Cluster stopped and cleaned up
```

Observed during this run:

- `nix run` emitted `unit2nix` evaluation warnings before the dogfood binary started.
- iroh emitted relay/DNS/IPv6 warnings during peer discovery and client connects in this environment.
- A follow-up `dogfood-federation start/status/stop` rerun found no `Endpoint dropped without calling Endpoint::close` lines in the command logs.
