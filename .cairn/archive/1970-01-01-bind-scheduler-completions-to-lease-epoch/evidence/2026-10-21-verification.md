# Scheduler assignment-token execution evidence — 2026-10-21

## Scope and ownership

Change: `bind-scheduler-completions-to-lease-epoch`, on isolated worktree branch `cairn/next-audit-20261021`, based on `d182f917f`. Immediate outcome: a stale completion cannot finish a later assignment when ownership returns to the same worker. Durable contribution: an assignment token bound to job, owner, and lease epoch, a typed stale-assignment denial, and an independent history checker. Molten fabric-simulation maintainers own the reference transition, the token encoding, and the checker.

## Executed pre-fix reproduction

A task-owned example used only the public `apply_reference_operation` API. It submitted a job, leased it to worker A at epoch 1, failed over to B at epoch 2, failed over back to A at epoch 3, then submitted A's epoch-1 completion:

```text
current_epoch=3 stale_completion=Applied recorded=Some("blake3:e23df7b7cd4b5a062d0742db8c30fe9fc53532c976a51f776c29c4c73f4b22e5") invariants=["single-authoritative-completion", "completion-requires-current-lease"]
```

The stale completion was accepted, recorded as authoritative, and the service reported `completion-requires-current-lease` as passed.

The recorded pre-edit baseline: `cargo test -p molten-core fabric_simulation` passed 27 and `cargo test -p molten --lib fabric_simulation` passed 14.

## Implementation

- `DistributedSchedulerOperation::Complete` now carries `assignment_token`. `scheduler_assignment_token(job_id, owner, lease_epoch)` hashes a domain-separated, delimiter-framed encoding with BLAKE3 and returns an opaque `blake3:` reference.
- The transition recomputes the token from current state after the duplicate, phase, and owner checks; a mismatch denies with `StaleAssignmentToken { job_id, expected_epoch }` and leaves state unchanged. An epoch overflow still denies `Failover` and `Lease` without mutation.
- `check_scheduler_completion_history` independently derives assignment owner and epoch from recorded `Submit`, `Lease`, and `Failover` operations, evaluates each completion token against the assignment that owned the job at that position, and reports `StaleAssignment`, `CompletionWithoutAssignment`, `DuplicateCompletion`, or `MalformedHistory`. It never reads a transition's reported invariant names.
- A test-local defective variant accepts the stale completion while reporting the same passing invariant names; the checker rejects that history at position 4.

Corrected public-path observation after the change, using the same temporary example before removal:

```text
current_epoch=3 stale_completion=denied StaleAssignmentToken { job_id: "job-a", expected_epoch: 3 }
independent_checker=Err(StaleAssignment { position: 4, job_id: "job-a", assignment_epoch: 3 })
```

## Executed checks

- `cargo test -p molten-core fabric_simulation`: 33 passed, zero failed. Covers current-token acceptance, stale owner-returns denial, token-reuse duplicate, token binding across job/owner/epoch, epoch overflow, checker detection of an accepted stale completion, and checker pass on matching histories.
- `cargo test -p molten --lib fabric_simulation`: 14 passed, zero failed, including the reference fixture that now carries the lease-issued token.
- `cargo fmt --all --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace --all-targets` passed 2,273 tests across 18 suites with zero failures; `cargo test --workspace --doc` passed 7 doctests.

## Preserved limits

- This change covers the reference scheduler only. It does not claim that a production scheduler, coordination-delivery service, or any other delivery path contains this defect, and it does not change coordination-delivery token semantics.
- Checker agreement does not prove whole-system correctness, external scheduler compatibility, or release eligibility.
- The checker evaluates recorded histories; it cannot observe a completion that a service rejects and never records. Its value is catching a defective service that accepts one.
- Epoch overflow is covered by a directly constructed leased state at `u64::MAX`; no reachable long-running path produced it.

## Source and Nix acceptance

Both configured Octet producers reported `clean` with zero findings, warnings and errors under unchanged profiles: [workspace status](2026-10-21-source/workspace/status.json) (`b3:bdaab55d…`) and [library status](2026-10-21-source/library/status.json) (`b3:2aad1f1f…`). Artifact import passed, [receipt](2026-10-21-source/workspace/artifact-ledger-receipt.preserves) `blake3:ab270c99969a4c53123611339343a34a1fb4d35be5abaec109cec34c62b44d2e`; the unchanged `strict-ci` gate passed, [receipt](2026-10-21-source/workspace/gate-receipt.preserves) `blake3:a5e2b186077913ef8eb753b806438fc30bdac682cf78891619db772e68758b65`; the [remediation plan](2026-10-21-source/workspace/remediation-plan.preserves) is `blake3:f39691e6df86b0009dd5511d7f2312d57223e8160c2560dcf97a874a434bd40b`.

`nix flake check -L --keep-going --no-update-lock-file` exited 0 with `all checks passed!` for the exported `x86_64-linux` checks. The hermetic nextest check realized 1,805 testcases with zero failures; its [CI receipt](/nix/store/m3ndx2sccbld66354anyiydww09mbhzz-molten-nextest/ci-test-run-receipt.preserves) records `decision="pass"`, source ref `blake3:6a34816d2e5870e60f593e18f3a72f76375cf5b4320ab150c29b2730ef0d3dca`, and raw receipt-file BLAKE3 `4548f1e959d6bf0848d9d9bf7fcd3674417057305eabb38fcf5e96673eb6ab0e`. That raw digest is not the canonical receipt reference. The check omitted `aarch64-darwin`, `aarch64-linux`, and `x86_64-darwin`.

## Native lifecycle acceptance

- Strict validation passed with 35 active changes, zero issues and zero findings; receipt `63b50e1b32e1b516cb568afa1cf5ee0f3de3c305394c644959e88aaac7903f64`.
- Proposal, design and tasks gates returned `PASS`: `42263be7729362656dbdd908f6ed208158b1fb4c2f1417b869b9581b6f533a02`, `3ea03d36c02d6abed5711d0ab39077f1f82ae890101eb348727762f8f562d84a`, `3d31d0ec5bfbbe83172666bf4892a2803e0454b236eddce3465d3d76725dc509`. The [completeness review](2026-10-21-review-completeness.json) returned `PASS`, receipt `da9d8238bde20018533dd927ecff1a11cb3b510ba5ba226414011b7db10290cd`.
- [Sync](2026-10-21-sync-receipt.json) merged the delta spec into `.cairn/specs/fabric-simulation/spec.md` with `blocked=false`, receipt `42245d50e7e5036ac2daa2ce71950e37e7036d67c1be929f4c46483f63aca6b9`.
- [Archive](2026-10-21-archive-receipt.json) reported `blocked=false`, receipt `38f418ca66fb6d3f6c0a94ec3a1896cd0230f6ca3e018622b9f1d021e5c57199`; the change now lives at `.cairn/archive/1970-01-01-bind-scheduler-completions-to-lease-epoch/`.
- Post-archive strict validation passed with 34 active changes, zero issues and zero findings, receipt `efaca5d3086626be94032b338eecb73e02c6540e215a490c754165a3ef5d54fa`.

These structural results do not replace the executed behavior evidence above.
