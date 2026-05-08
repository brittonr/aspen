## Context

The next highest-ROI action from the runtime-host readiness work is a fresh `nix run .#dogfood-local -- full` acceptance run. The run pushes current git `HEAD` into Aspen Forge, so implementation must commit any prerequisite fixes before claiming a dogfood result.

## Goals / Non-Goals

**Goals:** capture a fresh dogfood acceptance receipt for current `main`, prove the receipt is inspectable, and keep evidence secret-safe.

**Non-Goals:** redesign dogfood orchestration, broaden runtime-host promotion claims, or treat a failed run as accepted evidence.

## Decisions

### 1. Receipt-first acceptance

**Choice:** Acceptance is the versioned dogfood receipt plus operator readback, not console output alone.

**Rationale:** Receipts are durable, structured, and already part of Aspen's evidence contract.

**Alternative:** Chat-only transcript or raw logs were rejected because they are hard to audit and may contain sensitive details.

### 2. Failure becomes diagnostic evidence

**Choice:** If the full run fails, capture the receipt/diagnosis and leave implementation tasks open.

**Rationale:** A failed run can guide the next fix but must not be overclaimed as acceptance.

## Risks / Trade-offs

**Costly run** → Run after prerequisite status checks and keep evidence bounded.

**Secret leakage** → Store only redacted receipts/summaries and assert sensitive markers are absent from operator-visible evidence.
