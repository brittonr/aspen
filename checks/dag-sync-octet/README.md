# DAG-sync Octet scope

This workspace compiles the real `molten-core` DAG-sync source with the pinned Octet tool.

The scope includes the pure model, planner, positive tests, and negative tests. It excludes shell and transport effects.

This result does not replace the shell tests, live Iroh loopback, strict Clippy, Cairn gates, or repository release checks.

Run the focused gate from this directory:

```text
cargo octet check --workspace -- --all-targets --all-features
```
