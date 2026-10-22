# Outside-in fault track: entry points, boundaries, and runs

## Entry points the track drives

All steps are ordinary `molten` commands run from the repository root (`node
init` reads the workspace `Cargo.toml` for its Octet metadata fixture).

| Role | Command | Notes |
|---|---|---|
| Receiver setup | `node init`, `node run` | Node state root, identity, startup receipt. |
| Receiver admission | `node live-ticket-export`, `node live-peer-admit`, `node authority-grant-fixture` | The client peer is admitted and granted `status` before any send. |
| Request | `node control-request --operation status [--payload]` | Each distinct request uses a distinct payload and sequence; a reused sequence with another payload is denied (`duplicate-or-deny`). |
| Receiver listener | `node serve --live-iroh --live-ticket-out` | Binds loopback with relay disabled and a stable endpoint secret. It now writes the bound ticket once the endpoint is bound, through a staging file and a rename. Before this change it wrote the ticket only after the listener stopped, so no separate peer could use it. One run receives one control envelope, drains once, and exits. |
| Client | `node control-ingress-live-send` | Exits 0 for a denied send; the printed send receipt decision is the client's outcome. |
| Observation | `node control-history` (new) | Read-only: the latest delivery outcome per envelope, pending inbox requests, and processed requests with their decisions. |

Transport and storage surfaces reused: the live gossip endpoint
(`live_gossip_endpoint`: loopback bind, `RelayMode::Disabled`, memory address
lookup), bound live tickets, and the node state root. The live control path's
Redb store is `control/idempotency/delivery-idempotency.redb`. No parallel
configuration format was added: scenario declarations are typed Rust values
in `molten_core::outside_in`.

Existing harness tiers: `cluster harness-run` drives sequential one-shot node
commands over real Redb without Iroh between nodes. `cluster
fabric-transport-run` drives real Iroh between two children without node state.
Neither kills, pauses, or restarts a child on the same storage. The new
`outside-in` nextest profile is the declared tier for this track. Its tests
are named `live_outside_in_*`, so the deterministic profiles exclude them.

Consensus-path exclusion: `ScenarioPath::Consensus` is rejected by
`validate_scenario` before any process starts
(`live_outside_in_consensus_scenarios_never_start_a_process`). ChaosControl
and the live consensus reliability adapter own that path.

## Runs

`cargo test --test outside_in` (nextest unavailable locally: `cargo metadata
--all-features` panics on optional `rad://` sources). The 11 `live_outside_in_*`
scenarios passed in four consecutive runs, about 8.6 s per run with two test
threads:

| Scenario | Outcome |
|---|---|
| no-fault baseline | Agreement |
| kill-restart (listener-awaiting-request) | Agreement: post-restart read keeps request-1, three runs report one endpoint |
| pause-resume (listener-awaiting-request, 500 ms) | Agreement |
| partition-heal (transport-between-peers, 8 s) | Permitted divergence: retried-after-fault (partitioned attempt denied, healed retry delivered once) |
| lost-response (client-response) | Permitted divergence: lost-response-applied, duplicate-suppressed |
| consensus declaration | Declaration rejected, no process started |
| uninitialized listener root | Startup failed (listener), teardown complete |
| wall-clock, retained-output, process budgets | Budget exhausted with each kind, teardown complete |
| missing signal tool | Unsupported (`signal-STOP`) |
| corrupted processed receipts | Incomplete (final history) |
| substituted scratch path | Agreement, but scratch path retained: not a pass |

Non-claims: no production admission, no replacement of deterministic
simulation, and no performance characteristic. The partition is a loopback UDP
forwarder that drops the real Iroh datagrams. It is not a kernel network
namespace, VM, or WAN partition.

## Gates run

- `cargo test --test outside_in`: 20 passed (11 live scenarios plus the shared
  workspace-support tests).
- `cargo test -p molten-core --lib -- outside_in`: 14 passed (declaration
  validation and judgment, positive and negative).
- `cargo test --workspace --lib`: 1706 (`molten`) and 537 (`molten-core`)
  passed. `cargo test --test cliharness -- live`: 5 passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean. `cargo octet
  check`: clean (the test target included).
- `nix build .#checks.x86_64-linux.nextest-config`: built, with the
  `outside-in` profile filter `package(molten) & test(/outside_in/)`.
- Cairn strict validation: no spec or substance findings. Tracey guard: pass
  (2920 requirements, 986 referenced, 1934 inherited uncovered, 0 dangling).
