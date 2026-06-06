## Context

The architecture goal is Synit/SAM-style reactive services: demand, readiness, failure, supervision, restart, shutdown, and exposed service references are modeled as dataspace facts. Current code has a minimal runtime kernel and remote dataspace envelopes, but not the service lifecycle layer.

## Goals

- Define `service-manifest-v1`, `service-demand-v1`, `service-status-v1`, `service-lifecycle-receipt-v1`, `service-supervisor-v1`, and `service-cleanup-receipt-v1` records.
- Map service ownership to actor/session/live-ref authority contexts.
- Implement deterministic demand-driven start and shutdown over local dataspace assertions.
- Emit readiness, failure, degraded, restart, stopped, and cleanup assertions as canonical Preserves values.
- Add logical links/monitors independent from OS process parentage.
- Apply resource budgets to mailbox depth, restart rate, assertion count, turn count, and trace bytes.
- Clean up owned assertions, observers, live refs, pending effects, and service references when authority is revoked or a service terminates.
- Gate service supervision reports with replay-bound receipts that summarize restart/monitor/cleanup evidence while remaining non-authority operational evidence.

## Non-Goals

- No OS process supervisor compatibility.
- No Kubernetes/systemd model.
- No remote service discovery semantics beyond canonical envelopes.
- No unbounded restart loops.
- No service authority from a human-readable service name alone.

## Records

```preserves
<service-manifest-v1 "molten.service.manifest.v1"
  <service-id "svc:web">
  <owner <authority-context-ref>>
  <actor <actor-ref-or-artifact-ref>>
  <requires [<service-id> ...]>
  <provides [<assertion-pattern-ref> ...]>
  <restart-policy <restart-policy-ref>>
  <policy [<policy-ref> ...]>
  <resource [<resource-ref> ...]>
  <checks [<check "explicit-authority" "pass"> ...]>>
```

```preserves
<service-lifecycle-receipt-v1 "molten.service.lifecycle-receipt.v1"
  <operation "start"|"ready"|"fail"|"restart"|"stop"|"cleanup">
  <decision "pass"|"deny">
  <service <service-id>>
  <manifest <service-manifest-ref>>
  <turn-context <turn-context-ref>>
  <authority [<authority-receipt-ref> ...]>
  <resource [<resource-receipt-ref> ...]>
  <assertions [<assertion-ref> ...]>
  <diagnostics ["..." ...]>
  <checks [<check "owned-assertions" "pass"> ...]>>
```

```preserves
<service-supervision-gate-receipt-v1 "molten.service.supervision-gate-receipt.v1"
  <decision "pass"|"deny">
  <report <service-supervision-report-ref>>
  <suite <service-supervision-suite-ref>>
  <restart-decision <some "pass"|"deny"|"backoff">|<none>>
  <status-count N>
  <monitor-count N>
  <cleanup-count N>
  <diagnostics ["..." ...]>
  <checks [<check "supervision-report-replay" "pass"> <check "service-supervision-gate-is-not-authority" "pass"> ...]>>
```

## Runtime Algorithm

1. Observe service demand assertions.
2. Resolve the service manifest and required dependencies.
3. Admit startup through authority/resource/policy/effect handles.
4. Start the actor/service inside a transaction; pending readiness/failure assertions commit with the turn.
5. On failure, notify monitors, compute restart decision deterministically, and emit lifecycle receipt.
6. On stop/revocation, retract owned assertions/observers/live refs and emit cleanup receipt.
7. Gate reports by replaying the canonical supervision report and validating bound failure/status/lifecycle/restart/monitor/cleanup records; pass or deny with diagnostics without granting authority.

## Replay

Replay identity includes service manifest ref, demand assertion ref, dependency status refs, authority/resource refs, handler profile, restart policy ref, scheduler key, and recorded effect log. Replay fails at first divergent lifecycle decision, assertion set, monitor notification, restart attempt, or cleanup ref.
