# Committed subscriber conformance verification — 2026-10-26

## Scope, owner and repeatability

Parent revision: `fbc1c874d` (`Adopt verified standalone batch orchestration pilot`).
The immediate outcome is closure of the remaining committed-change-stream fault
case. The durable contribution is a required subscriber role in the reusable
world-fault profile, independently revalidated receipt evidence, and a regression
for lossless bounded paging within one revision. Molten contributors maintain
`committed_stream`, `world_faults`, the Nickel profile and its Nix checks.

The exercise calls the existing stream commit, subscription and dedup APIs. It
models subscriber restart in process, restoring only the acknowledged cursor.
The producer stream remains intact. No OS-process crash, persistent subscriber
store, physical power loss, cross-store transaction or whole-system safety is
claimed. Dataspace Observe, durable delivery, elections and leases are unchanged.

## Runtime proof

`2026-10-26-subscriber-smoke.log` records an executed throwaway Rust program linked
against the final `molten-core` library from the pinned Nix development shell:

```text
profile_ref=blake3:4879d730e10241f9e078655ab33cbf32aaee7a8d343a73776e80ac34bcfb0513
phase=before-response
restart_count=1
checkpoint_preserved=true
published_ordinals=[0, 1, 2]
resumed_ordinals=[0, 1, 2]
restart_before_ack=passed
dropped_event=failed
denial_diagnostics=[SubscriberObservationMismatch("post-restart-bound")]
Nickel/Rust profile identity: MATCH
```

The saved checkpoint precedes the multi-event publication. Receiving the first
one-event page does not acknowledge it; restoration replays that page and all
remaining events. Dedup returns exactly the three published identities. Deleting
one resumed delivery denies the result. The generated Nickel `rust_profile_ref`
was compared with the actual Rust profile identity, not inferred from source.

The helper and executable are task-owned scratch, not product code or a new test
harness contract. Their purpose is API execution proof; the permanent regressions
cover consumer-visible paging, replay and evidence-admission invariants.

## Exercised checks

Commands run in the dedicated repository worktree, inside `nix develop`, with
`TMPDIR=/home/brittonr/.cache/aspen-batch-20261025/tmp` and
`CARGO_TARGET_DIR=/home/brittonr/.cache/aspen-verify-20261023`:

- `cargo test -p molten-core --lib -- committed_stream world_faults`:
  **28 passed**, including the failing-before/passing-after paging regression.
- `cargo test -p molten --lib -- world_faults`: **3 passed**. The shell retains
  receipt-last sequencing, conservative owner failure, crossed receipt refusal,
  and identity changes when subscriber checkpoint evidence changes.
- `cargo clippy --workspace --all-targets -- -D warnings`: pass.
- `cargo fmt --all`: pass; later commit/publication hooks check formatting.
- Throwaway core API executable: pass and deliberate dropped-event denial.
- `world-faults-profile`, `world-faults-schema-inventory`, and
  `world-faults-octet-deny-all` Nix checks with `--builders ''`: pass. The focused
  Octet check reports `Status: clean`, `Findings: 0`, `Warnings: 0`, `Errors: 0`.
  The profile check evaluates the positive profile and all negative fixtures,
  checks source formatting and compares the generated export.
- `cargo test --workspace --lib`: **2,367 passed**, zero failures
  (`molten`: 1,738; `molten-core`: 629; other library targets: zero tests).
- `cargo octet check --artifact-dir target/octet-subscriber` and
  `cargo octet check -p molten --artifact-dir target/octet-subscriber-lib -- --lib`:
  both **clean**, zero findings, warnings or errors; pinned producer toolchain
  `nightly-2026-03-21-x86_64-unknown-linux-gnu`.
- Strict Cairn validation with the sibling provider policy: `valid: true`, with
  empty issues, findings, spec findings and substance findings. Full output is
  retained in `2026-10-26-strict-validation.json`; proposal/design/tasks gate
  receipts are retained in `2026-10-26-lifecycle-gates.json`.
- Changed documentation sections: balanced code fences; no broken relative links.

The v2 profile/receipt schema artifacts replace the active v1 artifacts, with
arities nine and fifteen. Historical archived receipts remain historical only.
The existing projection-only permanent test was removed rather than repinned;
Nickel export drift and exact Rust/Nickel identity are checked by the Nix rail
and executed smoke, while permanent tests cover behavior and negative admission.

## Source-policy repairs

The cursor contract uses an inclusive next ordinal. `subscribe` now takes a
portable `u32` page bound with checked conversion before allocation. Identity
length encoding zero-extends native-size bytes into the same eight-byte
big-endian representation; existing event/cursor identity domains and bytes are
unchanged on supported targets. Duplicate collection remains input-bounded and
preserves first-seen order without an eagerly allocated duplicate buffer.

The subscriber evaluator validates vector bounds before event inspection,
requires unchanged acknowledgement across restart and exact ordered published,
replayed and deduplicated sequences. Receipt validation recomputes diagnostics
and disposition independently; failed observations remain encodable as failed
evidence, but cannot be relabeled as a passing aggregate.

Initial failures and environment limits are preserved in
`2026-10-26-initial-failures.md`. Passing source or runtime observations grant no
content authority, mutation/cleanup permission, runtime enrollment, storage
correctness, production admission or release eligibility.
