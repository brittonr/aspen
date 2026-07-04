## Why

When a distributed or VM scenario fails, the useful debugging context is spread across topology, fixture, seed, fault plan, node receipts, logs, and command transcripts. Without a sealed repro bundle, failures are hard to replay or review, and sensitive diagnostics risk being copied without a clear redaction policy.

## What Changes

- Add sealed multinode failure repro bundles for simulation, local multiprocess, and VM evidence.
- Bind topology, scenario fixture, seed, scheduler, fault plan, commands, node summaries, receipts, diagnostics, redaction policy, and replay status.
- Add verify and unpack behavior that fails closed for tampered, missing, unsealed, private, or diagnostic-only evidence.
- Preserve logs as diagnostic-only attachments with explicit redaction or encryption policy.

## Impact

Distributed failures become portable and inspectable without turning diagnostics into pass evidence. Reviewers can verify what failed, replay deterministic cases, and understand why VM/live cases are non-replayable or unavailable.