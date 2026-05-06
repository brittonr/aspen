# Aspen patches for `nostr`

Vendored from crates.io `nostr 0.44.2`.

Local delta:

- Replace the wasm-only dependency on unmaintained `instant 0.1` with maintained
  `web-time 1.1`.
- Re-export `web_time::{Instant, SystemTime}` for `target_arch = "wasm32"` in
  `src/types/time/supplier.rs`.

Reason: `RUSTSEC-2024-0384` flags `instant` as unmaintained. The upstream Nostr
crate only uses it for wasm time shims, but Cargo/audit keep the target-specific
edge in `Cargo.lock`. Remove this patch once an upstream `nostr` release drops
`instant` or makes the same `web-time` replacement.
