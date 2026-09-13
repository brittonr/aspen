# Strict Octet observation for the native hashing candidate

## Result and source

The public strict gate rejects candidate `d98f7ff97024c3b8e16fca46f4a4a716e4304615`.
It reports 6,787 warnings and 330 critical findings.
Artifact presence, parsing, source scope, fingerprints, current metadata, and linkage checks pass.
Only `strict-status-clean` and `no-critical-findings` fail.
No status, warning, profile, baseline, allowance, or receipt changed to obtain acceptance.

Molten owns the consumer gate and its maintenance.
This round preserves a repeatable public import/gate observation instead of treating a successful lint command as acceptance.
It changes no product code. It does not combine the separate retry-domain candidate.

## Producer and consumer observations

| Step | Evidence |
| --- | --- |
| Current installed hook | `warning-only`, exit 0, 6,787 findings, no errors, 338 autofixable findings |
| Library-only Octet, task 1644 | `warning-only`, exit 0, 2,814 findings, no errors, 159 autofixable findings |
| Real corpus command, task 1645 | Exit 0. 13,682 objects and exact recorded replay command |
| Public artifact import, task 1655 | Exit 0. Decision `pass`, six imported artifacts |
| Public strict gate, task 1677 | Exit 1. Decision `deny`, with a canonical receipt |

Both Octet producer runs use the existing March compiler cohort.
The public consumer commands use the repository's unchanged May compiler cohort and repaired Cargo.
Each command has an eight-minute outer deadline. Build commands retain two Cargo jobs and the ordinary wrapper.
The import command recompiles dependencies and finishes its build in 5m 06s. The later gate build takes 1.15s.

The coordinator copies the hook artifacts byte-for-byte before the new commands.
The real corpus command receives all 1,363 sorted Rust paths under `src`.
Complete JSON read-back shows an exact match between that input list and the receipt's replay command.
The receipt lists 1,166 `source_paths`. An initial direct comparison with the full input list returns false.
The coordinator retains this distinction instead of claiming that the two fields have identical scope.

The corpus object-set hash is `b3:3f89527b2f14576d0121729f3a67b1647ce0e339bc4cdd8983c8730fabdf351f`.
Its caveats retain unresolved dependency/effect summaries and unmodeled macro contexts.
All 13,682 objects remain blocked from pure-cache eligibility. Object identity does not prove semantic correctness.

The import receipt is `blake3:6bc46e6dc3e62ab416f0bfb037b3eb9a8149deecadfa711bf7be594c3f843f47`.
The strict receipt is `blake3:4f02f3335e1fb953953d72ddf0bb756274727de4aad56f513e4be9e46bf3e9f5`.
The strict receipt retains these exact diagnostics:

```text
strict profile denies octet status `warning-only` with 6787 findings
unreviewed critical octet findings: 330
```

The successful import proves neither a clean status nor a passing gate.
No missing, malformed, stale, or unbound artifact explains this denial.

## Lifecycle boundary review

One read-only worker completes twelve reads/searches and returns an exhausted, source-backed report.
The coordinator verifies the cited F12 tasks/design and ChaosControl tasks/design.
F12 still has three open tasks for source checks, required tests/Nix, and Cairn evidence.
The ChaosControl package requires versioned producer contracts and an admitted cross-process guest for its dependent implementation.
Their current satisfaction is not established by this round.

The worker does not establish a mandatory full-Nix-before-sync edge or a universal human-approval gate.
The coordinator's F12 sync dry run, task 1666, reports `blocked: false`, `dry_run: true`, and `mutated: false`.
It proposes the four F12 requirement additions. It does not promote them.
Its receipt is `87016d0b46e78a73aa736568ebf2a333fd7dd3d7b9f0392ce7fbc87933cd7d64`.
Acceptance and review receipt identifiers remain empty.

The mechanical plan does not satisfy the outstanding source checks or authorize acceptance from an incomplete package.
No sync or archive execution occurs. The accepted specifications and historical Tracey inputs remain unchanged.

## Receipt handling and limits

A later status lookup for the worker's returned task number identifies an unrelated command.
No task control targets that entry. The worker's complete direct log and exit file remain intact.
The cause of the task-number mismatch remains unknown.

A separate audit verifies thirteen retained native-optimization Pueue exports against their exact command markers, worktree paths, groups, and terminal states.
The wrong-command and missing-task controls reject. Empty historical exports remain failed attempts, not terminal receipts.

This result establishes a current strict denial, not a strict pass or whole-Molten completion.
Full Nix, wider feature/runtime checks, legitimate lifecycle acceptance, and combined-source validation remain open.
No deployment, integration, push, or worktree removal occurs.
