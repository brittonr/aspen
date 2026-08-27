# Content-replication Octet scope

This workspace compiles the real content-replication functional core with the pinned Octet tool.

The scope includes positive and negative core tests. It excludes shell and adapter effects.

This result does not replace shell tests, Iroh tests, multiprocess tests, strict Clippy, Cairn gates, or repository release checks.

```text
cargo octet check --workspace -- --all-targets --all-features
```
