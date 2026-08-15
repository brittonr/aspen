# Design: resource reconciliation controllers

## Context

A Molten controller is a service that reconciles declared resources. This design adapts the Kubernetes controller/operator loop but preserves Molten's functional-core and imperative-shell boundary.

## Reconcile input

The pure reconcile core should receive:

- resource identity, generation, desired-state ref, and status summary;
- observed-state summary gathered by the shell;
- dependency resource summaries;
- policy and authority summaries;
- prior plan/effect/status receipt refs;
- retry/backoff state summarized as deterministic inputs.

## Reconcile output

The core returns one of:

- no-op with reason and status condition candidates;
- action plan with effect intents and required admission refs;
- terminal denial with diagnostics;
- retry plan with named backoff profile and next eligible turn.

The plan is not an effect. Adapter effects run only after admission and authority gates accept the plan.

## Work queue

The queue should coalesce events by resource ref and generation, preserve causal ordering where required, and record retry/backoff decisions. Named constants or policy values must define queue limits and backoff windows; hidden magic numbers are not acceptable.

## Receipts

A reconciliation receipt binds input refs, plan ref, admission receipt refs, effect receipt refs, status update refs, and terminal condition. Duplicate semantic commits or stale-generation plans deny before status can claim success.
