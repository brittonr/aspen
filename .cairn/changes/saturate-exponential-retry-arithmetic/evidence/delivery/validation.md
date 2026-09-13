# F12 delivery-boundary evidence

## Result and scope

Five new tests exercise `apply_delivery_request` through the actual delivery service.
The focused suite passed 21 tests, with no failures or ignored tests.
`retry-delivery-green.exit` records exit 0.

Molten coordination-delivery maintainers own these tests and their in-memory port adapters.
The tests preserve exact call order, committed state, requested timer batches, and error categories for repetition.
They do not establish live adapter, replay, strict-ci, or release acceptance.
The selected delivery profile still uses fixed backoff without jitter.

## Observed cases

| Case | Required observation |
|---|---|
| Fixed retry | Load precedes commit. Timer and status effects follow commit. The committed eligibility time matches the timer deadline. |
| Admitted exponential policy | Real enqueue, claim, and retry transitions reach attempt 63. Base four saturates at 128 rather than losing its high bit. |
| Duration denial | A retry delay of 11 exceeds the supplied profile limit of 10. The service reports `DeliveryIssue::ArithmeticOverflow` without state changes or effects. |
| Invalid delivery time | `u64::MAX` produces `DeliveryIssue::LogicalTimeRequired` before retry arithmetic or timer effects. |
| Timer I/O failure | A valid retry remains committed. The observation identifies failed timer references without accepted references or a false rollback. |

The exponential test uses a checked wide-integer oracle and distinct operation identities.
It derives expected state from the committed head after each request.
The final high-bit control distinguishes shift-count validity from value preservation.
This custom policy does not establish failure through the selected fixed profile.

Both denial cases compare the complete prior state, references, commit counters, timer requests, timer observations, and status references.
Only the load port runs after denial.
The duration case verifies error translation, not integer-overflow exposure.
Delivery-time admission rejects the large timestamp before deadline arithmetic.

## Baselines and later validation

The unchanged delivery suite passed before these additions.
The current fabric-time baseline passed 16 library tests and two CLI tests after one compilation-only timeout.
The added delivery suite then passed all 21 tests.

Workspace Clippy later rejected a constant assertion in the new time-bound test.
The assertion now uses a compile-time `const` block.
The next workspace Clippy run passed with `--all-targets` and `-D warnings`.
The all-feature metadata blocker still prevents required nextest acceptance.

No old-core run of the new delivery suite is recorded.
The earlier core regression suite supplies the red arithmetic evidence, not a red delivery-boundary result.
The later workspace run passed all 1,463 library tests, including the delivery tests.
That command reached its eight-minute deadline during a later native integration suite, exit 124.
It does not establish a passing workspace gate.

## Reproduction commands

Run these commands from the repository worktree.
The campaign used two Cargo build jobs and an eight-minute command deadline.
Its target directory was `/tmp/molten-completion-20260913-target`.

```console
nix develop --no-write-lock-file -c cargo test --locked -p molten --lib coordination_delivery::tests
nix develop --no-write-lock-file -c cargo test --locked -p molten fabric_time
nix develop --no-write-lock-file -c cargo clippy --locked --workspace --all-targets -- -D warnings
```

## Retained evidence

The campaign retains commands and raw logs under `.pi/molten-completion/logs/`.
Repository evidence includes lossless transcript archives, direct command exits, and BLAKE3 input identities.
The input identities describe the source bytes under test, not an authenticated release candidate.

This evidence does not complete the shell/adapter task.
Invalid jitter, exhaustion, true deadline overflow at the fixture boundary, canonical read-back, and replay divergence still need their required observations.
