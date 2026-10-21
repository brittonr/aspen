# Command-pattern reservation execution evidence — 2026-10-21

## Scope and ownership

Change: `adopt-command-pattern-resource-reservation`, on isolated worktree branch `cairn/next-audit-20261021`, based on `1bf357b24`. Immediate outcome: a resource reservation is one command evaluated against authoritative state, with bounded batch admission and expiry that does not silently free capacity. Durable contribution: a pure reservation contract, a shell that executes only decided effects, and operator-visible shortfall. Molten scheduler maintainers own the core transition, batch admission, capacity types, shell, and docs.

## Pre-change observation

The finding was a static source review of the reference scheduler's client-mediated placement sequence: capacity was read, a worker was selected separately, and an allocation was written later from a possibly outdated observation. This repository's fabric-time surface had no reservation command at all, so no public-path reproduction existed to execute; the correction introduces the command contract rather than repairing an existing call site. That limit is recorded below.

## Implementation

- `molten_core::fabric_time::reservation` owns the contract. `ReservationRequest` carries the job, the logical operation identity, `ResourceRequirements`, and the expected generation. `apply_reservation` evaluates one command against current state: a stale generation or insufficient capacity returns a typed rejection with the state unchanged; a committed command selects a worker, reserves and allocates the requirements, and records the reservation.
- `CapacityPool` keeps total, reserved, and allocated quantities distinct, and every transition checks that reserved never falls below allocated and neither exceeds the total.
- `admit_batch` enforces the declared item-count, byte-count, and waiting-time limits before evaluation, rejects an over-limit batch whole, and then evaluates each item as its own atomic command so one invalid item does not change its neighbours.
- `expire_reservation` marks an expired lease stale. Capacity is released only with an admitted enforcement or termination fact: an observed stop, a fencing takeover with termination authority, or a non-external work contract. Without one, the capacity stays counted and `reservation_status` reports the shortfall.
- `molten::fabric_time::ReservationRuntime` is the shell: it evaluates the command in the core and allocates only for a committed decision, so a rejection performs no effect.
- Documentation records that expiry does not stop an external job and that agreement proves no throughput or termination property.

## Executed checks

- `cargo test -p molten-core fabric_time`: 53 passed, zero failed, including the reservation contract regressions.
- `cargo test -p molten --lib fabric_time`: 50 passed, zero failed, including the shell regressions.
- `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings` pass.
- `cargo test --workspace --all-targets`: 1,639 library tests plus the remaining suites passed with zero failures; `cargo test --workspace --doc`: 7 doctests passed.
- Both configured Octet producers reported `clean` with zero findings, warnings and errors: [workspace status](2026-10-21-source/workspace/status.json) and [library status](2026-10-21-source/library/status.json). Artifact import [receipt](2026-10-21-source/workspace/artifact-ledger-receipt.preserves) `blake3:40150c633ad373ee975247decd66565b8d48de3a2bb271112099e77867574bf4`; unchanged `strict-ci` gate [receipt](2026-10-21-source/workspace/gate-receipt.preserves) `blake3:ddeed39279e991ee5613543ca5f7299aac975b3580ab9c2eba2c73421e894843`; [remediation plan](2026-10-21-source/workspace/remediation-plan.preserves) `blake3:3786d9467e6c292c43d03af398be75c24d9f6c0496b33c62db9db2593af32043`.
- `nix flake check -L --keep-going --no-update-lock-file` exited 0 with `all checks passed!` for the exported `x86_64-linux` checks. The hermetic nextest check realized 1,816 testcases with zero failures; its [CI receipt](/nix/store/dknskydh0248w4qda927x4y0vjsbc1pw-molten-nextest/ci-test-run-receipt.preserves) records `decision="pass"`, source ref `blake3:a34fc95776bc318720e9971c746eb25c7eb50241a3028fb3525a8df4e7bff5c3`, and raw receipt-file BLAKE3 `9ad6ee0b1b136196bb0f9592c43f22926d96a1402336ee6f10e03be33e46d412`. The raw digest is not the canonical receipt reference. The omitted incompatible systems were `aarch64-darwin`, `aarch64-linux`, and `x86_64-darwin`.

Preserved failures during this work: the first Octet pass reported twenty-seven findings in the new shell module, repaired by moving the shell file under `shell/`, qualifying owner paths at their use sites, and removing panicking expectations. A later Nix run denied the inherited traceability debt because the previously synced promotion delta left `molten.promotion_identity.validation` without a direct evidence reference; a verification marker now binds it, and the gate passes.

## Preserved limits

- The pre-change gap was a static source review with no executed counterexample, and this repository had no prior reservation command to reproduce. The regressions below are the executed evidence for the new contract, not a before/after comparison of an existing path.
- The command contract is a new admission surface. It does not claim throughput improvement, scheduling fairness beyond the admitted profile, or the ability to stop an external job.
- Worker selection is deterministic over the admitted worker roster; it does not model heterogeneous worker capability beyond the declared unit and byte bounds.
- Absence of an enforcement fact keeps capacity counted, which can report a shortfall that only an external stop can clear. That visibility is the intended behaviour, not a leak.

## Native lifecycle acceptance

- Strict validation passed with 32 active changes, zero issues and zero findings; receipt `3af2b4544aeecb58c1f5a5da27536e3977d6b679080af4ee571f1e17a144962d`.
- Proposal, design and tasks gates returned `PASS`: `47fd50086721f252eb42d714a936e82db2d0067e2b90cfa478eb32b54783189d`, `06addb5488b4de6e11d0523c420fc1f2a85d38e8f23fb183764c2a61ff36df2f`, `2fac8650acc549e5f4a202510810601747a8c8c0f5a2b8b94137707c723f4cd7`. The [completeness review](2026-10-21-review-completeness.json) returned `PASS`, receipt `478c8f9ba006126d5664698f808014cf28a245ef28d702b6295856410e9c5e75`.
- [Sync](2026-10-21-sync-receipt.json) merged the delta spec into `.cairn/specs/fabric-time-scheduling/spec.md` with `blocked=false`, receipt `40780a28ba5e02674bf668981b0ae411b49ad5fb80b674d1c291c43f567c5090`.
- [Archive](2026-10-21-archive-receipt.json) reported `blocked=false`, receipt `c0327b1ac3a80693a3acbe1db50c35bf00c614444f27e9b7b1832a17263e6ed6`; the change now lives at `.cairn/archive/1970-01-01-adopt-command-pattern-resource-reservation/`.
- Post-archive strict validation passed with 31 active changes, zero issues and zero findings, receipt `052c40f1eda98ac255222f6a2f556cb3d76b09028afcd82f42c295f463a64c0c`.

These structural results do not replace the executed behavior evidence above.
