# Tasks: Bind scheduler completions to the lease epoch

## State machine change

- [ ] [serial] Add the assignment token to the `Complete` operation, derive it at lease time from job, owner, and epoch with a domain-separated BLAKE3 encoding, and reject mismatches with a typed `StaleAssignmentToken` issue. r[molten.fabric_simulation.assignment_token]
- [ ] [parallel] Add positive fixtures for the current-token completion and negative fixtures for the owner-returns trace, wrong-token rejection, token reuse after completion, and epoch overflow. r[molten.fabric_simulation.assignment_token]
- [ ] [parallel] Update every `Complete` call site in reference fixtures and tests to carry the lease-issued token. r[molten.fabric_simulation.assignment_token]

## Independent check

- [ ] [serial] Add a pure checker that derives assignment epochs and ownership from the recorded transition history and evaluates every authoritative completion against its owning assignment. r[molten.fabric_simulation.stale_completion_check]
- [ ] [parallel] Add a deliberately broken scheduler variant that skips the token check and prove the checker fails its history while the service reports its invariants as passed. r[molten.fabric_simulation.stale_completion_check]
- [ ] [serial] Retain the owner-returns trace as a permanent regression and keep the checker result independent of reported invariant names. r[molten.fabric_simulation.stale_completion_check] r[molten.fabric_simulation.assignment_token]

## Validation and closeout

- [ ] [serial] Run focused reference-service and fabric-simulation tests, formatting, Clippy, Octet, Cairn validation, and the proposal, design, and tasks gates. r[molten.fabric_simulation.assignment_token] r[molten.fabric_simulation.stale_completion_check]
- [ ] [serial] Retain the reference-only and no-whole-system-correctness non-claims before sync or archive. r[molten.fabric_simulation.stale_completion_check]
