# Guarded lifecycle input wiring

This checkpoint adds explicit evidence paths. It does not approve a runtime cohort.
Tasks 4 and 5 remain open.

## Boundary changes

- `node run` accepts paired startup policy and bundle paths.
- Content serving accepts the same pair and re-reads evidence before state or listener effects.
- Plain serving and `--live-iroh` reject supplied startup evidence instead of ignoring it.
- Verification and lifecycle inputs share descriptor-first loading and strict snapshot evaluation.
- Even a passing snapshot fails lifecycle admission with `startup-evidence-real-cohort-not-approved`.
- Missing content-side evidence fails in tests and production. The test-only startup fallback cannot authorize serving.

The review caught an initial implementation that returned a passing snapshot directly to startup.
That route was not committed or used for a live node. The retained guard now prevents that promotion.
Unit fixtures establish wiring behavior only, never actual execution or source-to-binary provenance.

## Verification scope

Focused validation uses existing Rust 1.97.1 with `RUSTC_BOOTSTRAP=1`.
This is not the declared May-26 production compiler.
The March-21 formatter formats the changed Rust owners.
No compiler, Mantle, Darkhttpd, package toolchain, or Stage0 build is part of this checkpoint.

Earlier commands piped Cargo output through `grep`, `tail`, or `head` without preserving Cargo's exit status.
Those task exit codes are not independent success evidence.
A later battery runs Cargo directly, preserves complete output, and stops on failure.
The broader daemon batch was stopped before daemon execution because that selection includes host-network tests.
The replacement selection uses local lifecycle behavior and pre-effect denials only.

Final focused validation: Pueue tasks 10687 and 10691 completed the direct command chains.
They passed 52 tests: adapter 7, quality 36, local daemon 2, content denial 4, and CLI 3.
Library Clippy with `-D warnings`, selected-file rustfmt checks, and `git diff --check` also passed.
The first local-daemon filter in the earlier run matched zero tests; the final run names the actual lifecycle test.

The read-only report remains `verification-only`, with both execution and startup authority false.
No new normal-node VM, transfer, seedless restart, native replay, physical deployment, or release promotion is claimed.
