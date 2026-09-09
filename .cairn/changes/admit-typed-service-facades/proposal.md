# Proposal: Admit typed service, state-machine, and task facades

## Why

The system-extension interface supplies the ingredients of a safe long-running service: lifecycle, requests, messages, streams, timers, health, checkpoints, and recovery. But an application author assembles those ingredients by hand today. Elixir's `GenServer` and Erlang's `gen_statem` show the leverage of a standard structure: the developer supplies application callbacks; the behaviour owns the surrounding service loop. The transferable lesson is a small, typed facade over the existing interface, not a new runtime.

## What Changes

- Add three library-level facade profiles over the existing system-extension interface: `Service` (long-lived state owner handling commands and queries), `StateMachine` (legal events depend on the current phase, with bounded state timeouts and explicit bounded postponement), and `Task` (a finite unit of work with an explicit completion result).
- The application author's model is `current state + admitted event -> next state + replies + effect intents + timer intents`; the facade lowers that result into existing canonical envelopes, admission checks, lifecycle transitions, and evidence.
- The facades MUST NOT introduce an alternative route to I/O, a second state store, or runtime implementations beside the existing ones.
- Timers scheduled through the facades are incarnation-fenced (per `fence-runtime-local-work-across-restarts`); a timer for an earlier state or earlier incarnation MUST NOT cause a transition in a replacement runtime.
- State-machine postponement MUST be explicitly bounded; unbounded postponement is an admission denial.
- Establish an explicit serialization contract: a service-level concurrency limit above one MUST NOT permit concurrent mutation of one actor's logical state; parallel work is partitioned or returns through a controlled commit.
- Keep ordinary pure functions ordinary: facades are opt-in profiles, and nothing in this change turns validation, hashing, policy evaluation, or planning into actors.

## Impact

- **Files**: a new facade library layer over `molten-core` system-extension models, plus facade fixtures and tests. No changes to the core transition semantics.
- **Testing**: each facade on the happy path (command, event, completion); negative cases (illegal event for phase, unbounded postponement, stale-state timer, concurrency-limit violation attempting shared-state mutation); equivalence between facade-lowered envelopes and hand-written envelopes.
- **Non-goals**: no new runtime, mailbox, scheduler, store, or authority system; no claim of GenServer or gen_statem API compatibility; no retrofitting existing extensions.

## Dependencies

- `fence-runtime-local-work-across-restarts` for timer incarnation fencing.
- `admit-recovery-group-supervision` for task completion semantics (a completed `Task` must not restart).
