# Cargo audit advisory inventory

Status: captured
Captured: 2026-05-06

## Commands

- command: `cargo audit -n --ignore RUSTSEC-2023-0071 --json > /tmp/aspen-cargo-audit.json || true`
- command: `nix build .#checks.x86_64-linux.audit --no-link -L`

## Initial vulnerable clusters

Initial local audit JSON reported 23 vulnerabilities and {'unmaintained': 8, 'unsound': 6} warnings.

- `astral-tokio-tar`: 1 advisory/advisories — RUSTSEC-2026-0066
- `aws-lc-sys`: 2 advisory/advisories — RUSTSEC-2026-0048, RUSTSEC-2026-0044
- `rustls-webpki`: 4 advisory/advisories — RUSTSEC-2026-0104, RUSTSEC-2026-0099, RUSTSEC-2026-0049, RUSTSEC-2026-0098
- `tar`: 2 advisory/advisories — RUSTSEC-2026-0068, RUSTSEC-2026-0067
- `wasmtime`: 14 advisory/advisories — RUSTSEC-2026-0087, RUSTSEC-2026-0020, RUSTSEC-2026-0095, RUSTSEC-2026-0021, RUSTSEC-2026-0091, RUSTSEC-2026-0088, RUSTSEC-2026-0086, RUSTSEC-2026-0093, RUSTSEC-2026-0006, RUSTSEC-2026-0092, RUSTSEC-2026-0094, RUSTSEC-2026-0096, RUSTSEC-2026-0089, RUSTSEC-2026-0085

## Classification

| Cluster | Owning surface | Initial disposition | Remediation |
| --- | --- | --- | --- |
| `wasmtime 36.0.3` | optional plugin/WASM runtime lock graph (`plugins`) | vulnerable sandbox/runtime cluster; no blanket ignore | updated lock graph to `wasmtime 36.0.7` and matching 36.0.7 internal crates |
| `aws-lc-sys 0.38.0` | TLS/cert validation via rustls/aws-lc paths | certificate/CRL validation advisory cluster | updated via `aws-lc-rs 1.16.3` / `aws-lc-sys 0.40.0` |
| `rustls-webpki 0.103.9` | TLS/cert validation via rustls paths | certificate/name/CRL validation advisory cluster | updated to `rustls-webpki 0.103.13` |
| `tar 0.4.44` | archive extraction / snix materialization lock graph | archive unpack/chmod/PAX advisory cluster | updated to `tar 0.4.45` |
| `astral-tokio-tar 0.5.6` | upstream `snix` NAR/archive path | fixed upstream in `0.6.0`, but current `snix` git pin still requires `^0.5.6` | bounded advisory-specific ignore for RUSTSEC-2026-0066 with removal trigger when snix moves to `astral-tokio-tar 0.6.x` |

## Warning-only advisories after remediation

Warnings remain warning-only and are not newly ignored as vulnerabilities. They are tracked separately from this vulnerability-remediation change:

- `unmaintained` (8):
  - RUSTSEC-2023-0089 `atomic-polyfill 1.0.3` — atomic-polyfill is unmaintained
  - RUSTSEC-2025-0141 `bincode 1.3.3` — Bincode is unmaintained
  - RUSTSEC-2024-0384 `instant 0.1.13` — `instant` is unmaintained
  - RUSTSEC-2025-0119 `number_prefix 0.4.0` — number_prefix crate is unmaintained
  - RUSTSEC-2024-0436 `paste 1.0.15` — paste - no longer maintained
  - RUSTSEC-2024-0370 `proc-macro-error 0.4.12` — proc-macro-error is unmaintained
  - RUSTSEC-2024-0370 `proc-macro-error 1.0.4` — proc-macro-error is unmaintained
  - RUSTSEC-2023-0081 `safemem 0.2.0` — safemem is unmaintained
- `unsound` (6):
  - RUSTSEC-2023-0086 `lexical-core 0.8.5` — Multiple soundness issues
  - RUSTSEC-2026-0002 `lru 0.12.5` — `IterMut` violates Stacked Borrows by invalidating internal pointer
  - RUSTSEC-2026-0097 `rand 0.8.5` — Rand is unsound with a custom logger using `rand::rng()`
  - RUSTSEC-2026-0097 `rand 0.9.2` — Rand is unsound with a custom logger using `rand::rng()`
  - RUSTSEC-2023-0056 `vm-memory 0.10.0` — Default functions in VolatileMemory trait lack bounds checks, potentially leading to out-of-bounds memory accesses
  - RUSTSEC-2024-0002 `vmm-sys-util 0.11.2` — `serde` deserialization for `FamStructWrapper` lacks bound checks that could potentially lead to out-of-bounds memory access
