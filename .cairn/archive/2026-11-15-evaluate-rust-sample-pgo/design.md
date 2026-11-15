## Cohort and ownership

Use nightly-2026-08-20, Rust 1.100.0-nightly commit f7d782a3be46d6bb4b9792fe69a61db389ba1769, separately from default nightly-2026-05-26 and Octet. Compiler/component SHA-256 pins come from the dated official manifest. LLVM converter source is the compiler submodule commit 21cf28432798952d942bacc6bcee3a328faa3638. Both variants use identical flags including explicit nightly profiling debug information; only sample-profile consumption differs. Reuse the existing immutable unit2nix dependency graph and overrides; never mutate plans or locks or disable source guards.

## Workload and gates

Use the existing native harness actor/assert/send/clock replay path. Training and held-out inputs have disjoint labels/seeds/payloads. Training alone feeds bounded external perf LBR sampling. Held-out evaluation includes admitted success, capability denial, suite-step exhaustion, and invalid actor errors. Harness-generated Preserves reports/failures remain authoritative; byte-identical canonical renderer output is a stricter additional equality check, not a replacement identity scheme. Both binaries replay each successful/denied report including its recorded effects, validating canonical trace, receipt, output, and final-state refs. Failure artifacts must match exactly; no final state is invented for a preflight failure.

## Measurement and decision

Predeclare 30 paired repetitions after three warmups, alternating order; use per-invocation client wall time, user/system CPU, max RSS, binary size, end-to-end Nix realization wall/CPU/RSS (including cache effects), and measured host conditions. Retain every invocation, return code, command, stdout/stderr, and artifact. Report retained memory as unsupported for an exited CLI process, not zero; do not infer p99 from 30 samples. Keep requires a conservative paired 95% confidence lower bound above 5% held-out wall improvement with no case median regression above 2%, peak RSS growth above 5%, or binary growth above 10%, and all semantic gates. Otherwise measured stop/revise/inconclusive only; setup failure is blocked.

## Failure and staleness

Fresh output directories only. Bind source store closure, Cargo.lock/build-plan hashes, compiler/converter executable hashes/version output, fixed flags, datasets, baseline ELF build ID/hash, perf raw bytes and build-ID association, converted sample hash, and candidate derivation to one manifest. A captured profile is never imported from an arbitrary path. Missing perf permissions/events/LBR, empty/mismatched profiles, timeouts, conversion/build failures, or semantic drift prevent a valid comparison. No `.fxt`, allocator instrumentation, silent event fallback, RUSTC_BOOTSTRAP, or synthetic admission is allowed.
