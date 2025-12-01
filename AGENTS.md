# Repository Guidelines

## Project Structure & Module Organization
Per `GEMINI.md`, Aspen centers on four Rust modules: `orchestration` (control plane flows), `distributed` (iroh + hiqlite primitives), `traits` (service contracts), and `types` (shared data). Keep Raft experiments in `openraft/`, architecture notes plus the `docs/iroh-examples` reference tree under `docs/`, and place integration scenarios in `tests/` named for the subsystem they cover.

## Build, Test, and Development Commands
- `nix develop` (or `nix-shell -p <package>`) enters the pinned dev shell from `flake.nix`.
- `cargo build` compiles the crate, while `cargo check` catches regressions quickly between edits.
- `cargo nextest run` is the primary runner; use `cargo test` only for fast spot checks.
- Keep `context7 mcp serve` running when working with the MCP integrations mentioned in GEMINI.

## Coding Style & Naming Conventions
Tiger Style governs every change: favor simple, explicit control flow, avoid recursion, and set fixed limits on loops or queues. Keep functions under ~70 lines, use explicitly sized integers, and statically allocate long-lived data where possible. Treat compiler warnings as errors, assert arguments and invariants aggressively, and document tricky constraints with concise comments. Stick to idiomatic Rust naming (`snake_case` functions/modules, `PascalCase` types) and format through `cargo fmt`.

## Testing Guidelines
GEMINI emphasizes property-based testing via `proptest`, so each module should surface generative tests that encode invariants. Distributed code must run inside deterministic simulation (`madsim`) to expose race conditions faster than Jepsen-style suites, and tests should assert both success and failure paths (e.g., `test_actor_respects_lease_limit`). Capture simulator seeds or failure traces in accompanying docs when relevant.

## Commit & Pull Request Guidelines
Zero technical debt is part of Tiger Style, so commits should be small, intentional, and reversible. Use descriptive Conventional-Commit-style subjects (`feat(distributed): add dag sync hooks`) and explain how the change defends the stated safety/performance goals. PRs need links to roadmap work, `cargo check`/`cargo nextest` evidence, simulator runs when applicable, and explicit notes about operator actions or config shifts.

- Treat each milestone as a distinct, minimal commit; land incremental progress frequently rather than batching unrelated work.
- After completing a milestone, immediately update `plan.md` to reflect the new progress and remaining steps.

## Environment & Security Notes
GEMINI assumes the Nix dev shell plus the pinned Rust channel, so add tools via `nix-shell -p` instead of ad-hoc installs. Keep secrets out of the repo by leaning on local environment overlays. When adjusting dependencies such as `iroh` or `hiqlite`, record rationale in `docs/` and cross-check the upstream resources referenced at the end of GEMINI.

## Ractor Cluster Notes
We're adopting `ractor` with the `cluster` feature plus `ractor_cluster`/`ractor_actors` so we can host distributed actors. Key reminders:
- `NodeServer` owns the listener plus the per-peer `NodeSession` actors and is the single entry point to bring a node online. Every host must run one.
- Nodes authenticate via Erlang-style “magic cookies”; make sure peers share the same cookie before calling `client_connect`/`client_connect_enc`.
- **Bring Your Own Transport:** implement `ractor_cluster::ClusterBidiStream` for your connected stream (just split into `AsyncRead`/`AsyncWrite` halves) and feed it to the node server with `NodeServerMessage::ConnectionOpenedExternal`/`client_connect_external`. The on-wire protocol stays the same (len-prefixed, prost-encoded frames), so you don’t touch auth or PG sync logic. This makes QUIC/WebSocket/in-memory transports possible alongside the default TCP/TLS paths. For example:
  ```rust
  use ractor_cluster::{ClusterBidiStream, BoxRead, BoxWrite};
  use tokio::io::DuplexStream;

  struct MyDuplex(DuplexStream);

  impl ClusterBidiStream for MyDuplex {
      fn split(self: Box<Self>) -> (BoxRead, BoxWrite) {
          let (r, w) = tokio::io::split(self.0);
          (Box::new(r), Box::new(w))
      }
      fn peer_label(&self) -> Option<String> { Some("duplex:peer".into()) }
      fn local_label(&self) -> Option<String> { Some("duplex:local".into()) }
  }

  // elsewhere
  // node_server.cast(NodeServerMessage::ConnectionOpenedExternal {
  //     stream: Box::new(MyDuplex(conn)),
  //     is_server: false,
  // })?;
  ```
- Message enums for actors that can be remoted must derive `RactorClusterMessage` (or implement `ractor::Message` + `BytesConvertable` manually) so they serialize across the wire. Use `#[rpc]` on variants that expect replies.
- `ractor_actors` ships a set of helper actors (watchdog, broadcaster, filewatcher, etc.)—enable only the features you need to keep the dependency surface minimal.
