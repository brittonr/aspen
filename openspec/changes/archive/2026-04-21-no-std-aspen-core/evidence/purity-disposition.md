# Purity disposition for alloc-only `aspen-core`

This inventory covers the alloc-only families that remain visible without `feature = "std"`:

- `circuit_breaker`
- `cluster`
- `constants`
- `crypto`
- `error`
- `hlc`
- `kv`
- alloc-safe `prelude`
- `protocol`
- `spec`
- `storage`
- `traits`
- `types`
- `vault`
- `verified`
- alloc-safe optional `sql`

Shell-only modules (`app_registry`, `context`, `transport`, `simulation`, `utils`, `layer`) are intentionally excluded because they are now `std`-gated and do not participate in the alloc-only contract.

## Audit summary

| Category | Disposition | Notes |
| --- | --- | --- |
| Explicit randomness / configuration inputs | explicit only | `circuit_breaker` now takes explicit millisecond timestamps; `sql` limit/timeout helpers and `verified::scan` pagination/limit helpers already accept explicit values rather than ambient state. |
| Ambient environment reads | absent | No `std::env` / `env::` reads in the alloc-only families. |
| Process-global state | absent | No `process::id`, `OnceLock`, `lazy_static!`, `thread_local!`, or `static mut` in the alloc-only families. |
| Hidden runtime context | absent | Pure helpers take values/traits explicitly. `protocol::ProtocolCtx`, `traits::*`, and `sql::SqlQueryExecutor` are contract boundaries, not hidden singleton/runtime lookups. |
| I/O | absent | No filesystem, socket, or stream I/O in the alloc-only families. |
| Runtime-bound async bodies | absent except contract traits | Alloc-only async appears as trait signatures (`traits.rs`, `sql.rs`) or test code only. No alloc-only production module contains runtime-coupled async bodies. |
| System calls | absent | No filesystem/process/network syscalls in the alloc-only families. |
| Implicit randomness sources | absent | No `rand` / `getrandom` calls in the alloc-only families. |

## Proof notes

Searches run against the alloc-only family set after shell gating landed:

- `rg -n 'std::env|env::|process::|Command|spawn\(|std::fs|fs::|std::io|io::|tokio|iroh|reqwest|tracing|rand|getrandom|OnceLock|lazy_static|thread_local!|static mut' crates/aspen-core/src/{circuit_breaker.rs,cluster.rs,constants,crypto.rs,error.rs,hlc.rs,kv,prelude.rs,protocol.rs,spec,storage.rs,traits.rs,types.rs,vault.rs,verified,sql.rs}`
  - Result: only test-only `#[tokio::test]` lines in `traits.rs` plus comment hits in `protocol.rs` / `constants/mod.rs`; no production alloc-only runtime calls.
- `rg -n 'async fn' crates/aspen-core/src/{circuit_breaker.rs,cluster.rs,constants,crypto.rs,error.rs,hlc.rs,kv,prelude.rs,protocol.rs,spec,storage.rs,traits.rs,types.rs,vault.rs,verified,sql.rs}`
  - Result: alloc-only async is limited to contract trait signatures and test code.
- `rg -n 'std::|tokio|iroh|rand|env::|process::|fs::|io::' crates/aspen-core/src/verified`
  - Result: no matches.

## Follow-up

- `3.1a` remains for any remaining alloc-only API that still needs explicit randomness/config refactoring beyond the already-landed `circuit_breaker` conversion.
- `3.1b`, `3.1c`, and `3.11` now have mechanical proof in `openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/source-audit.txt`; future work only needs to reopen them if new alloc-only APIs are added.
