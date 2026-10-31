# Proposal: Host admitted recovery groups in the public node

## Why

`exercise-service-recovery-composition` cannot honestly close its live twin while a test owns an in-memory supervisor. Existing system-extension owners supply pure recovery/readmission laws and actual native callback execution, but the public node does not compose those mechanisms or retain group history/current claims after process loss. The user authorized this separate prerequisite on 2026-10-29.

## What changes

- Add a capability-rooted node hosting shell with durable group membership, ordered recovery intents, restart-window history, instance generation/incarnation observations, durable work and current delivery claims.
- Reuse actual native `Supervisor` lifecycle callbacks and admitted generation replacement. Keep planners pure and runtime effects behind an explicit port.
- Expose admitted public node install, status, supervise, submit/readmit/deliver and upgrade operations. Attach hosting only after source/startup admission; never weaken that gate.
- Persist intent before lifecycle effects and report interrupted operations as uncertainty rather than retry permission. Reopen the real ledger after process loss.
- Enable the composition live twin using public node processes and the existing deterministic assertion vocabulary.

## Non-goals

No new harness-owned supervision mechanism, fake executable identity, fake durable store, implicit authority from artifact possession, production acceleration, distributed failure detector, or exactly-once external-effect claim. A canonical receipt is evidence, not permission. No source-gate daemon change is authorized by this prerequisite.

## Acceptance

Real public-node processes must retain restart budgets and current work claims across forced loss, reject old incarnation/generation deliveries, readmit durable work explicitly, terminate storms, and perform admitted drain/checkpoint/transformation/activation while an independent sibling progresses. Both tracks must reject meaningful broken variants. Validation and publication remain separate gates.
