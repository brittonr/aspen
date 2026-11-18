# Verification 2026-11-18

Scope: `pilot-wrpc-component-transport` with feature `component-rpc-telemetry`,
in the repository-pinned `nix develop` shell, Cargo dev profile with debug
information disabled. Source: Molten `ed52b83c0` (Tiger Style repairs) on top of
`43482e4b1` (added fixture denials). Loopback fixture fabric and fixture
credentials only.

| Check | Command | Result |
| --- | --- | --- |
| Focused tests | `cargo test --locked --features component-rpc-telemetry --test componentrpc` | 4 passed |
| Pilot and shared-verifier unit tests | `cargo test --locked --features component-rpc-telemetry --lib -- component_rpc:: wasm_component::evidence::materialization::published` | 14 passed |
| Clippy | `cargo clippy --locked --features component-rpc-telemetry --all-targets -- -D warnings` | no findings |
| Octet | `cargo octet check -p molten --artifact-dir DIR -- --all-targets --features component-rpc-telemetry` | `clean`, 0 findings (at `43482e4b1`: `warning-only`, 56 distinct findings) |
| Spacewasm pilot mount | `cargo check --locked --features spacewasm-pilot --bin molten` | compiles (mount path moved) |
| Real-bundle run | `run-fixture`, `show`, `replay`, `telemetry-fixture` and `telemetry-show` on the bundles of the [admission run](real-bundle-admission-run-2026-11-18.json) | all five commands exited 0 with image `51e8840b31d1d89f…`; the same 38 negative cases as the admission run were rejected in the same order, replay output was identical, the three telemetry denials held and one span was exported; no staging leftovers |
| Cairn | `cairn validate --root . --policy ../cairn/cairn-policy/generated/cairn-policy.json --strict` | valid, 0 issues, 0 findings (after the status change to `current`) |
| Lifecycle gates | `cairn gate proposal`, `design` and `tasks` for this change | 0 issues each |

## Octet repairs

The configured Octet scope (`-p molten -p molten-node-host`, default features)
does not compile `component-rpc`, so the pilot had never been checked. The first
feature-enabled run at `43482e4b1` reported 56 distinct findings. `ed52b83c0`
repairs them without changing behaviour: module directories hold at most seven
files (shell and fixture modules moved under `#[path]` mounts, public module
paths unchanged), module imports became qualified paths, four types that
repeated their module were renamed (`transcript::Call`, `journal::Recorder`,
`diagnostics::Queue`, telemetry `bundle::Emitter`), the fixture UCAN limits are
stated with the library's default values, numeric conversions are checked,
collections reserve capacity, enum matches list their variants and three long
functions were split. One finding is allowed with a reason:
`wrpc_runtime_wasmtime::SharedResourceTable` has a private field, so `Default`
is its only constructor.

## Not established

- The configured Octet scope and hosted CI still build default features, so
  nothing reruns the feature-enabled Octet check automatically.
- Loopback only: no remote or cross-host RPC, transport security, production
  credential admission or release evidence.
- The Iroh overlap decision required for promotion is not made; the pilot stays
  experimental.
