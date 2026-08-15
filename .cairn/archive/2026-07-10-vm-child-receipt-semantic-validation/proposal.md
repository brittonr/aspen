## Why

The VM run receipt binds child refs for protocol, reconcile, ack, remote service, job, coordination, soak, and fault validation evidence. Current validation can prove top-level topology and run shape, but it should also parse key child receipt contents so a stale or wrong-node child ref cannot satisfy a cluster-testing claim by ref presence alone.

## What Changes

- Extend VM evidence validation to parse declared child receipt classes and check their semantic bindings.
- Require live-control child receipts to bind expected sender, receiver, peer, operation id, ticket, admission, authority, ingress, queue, dispatch, reconcile, ack, and protocol refs.
- Require service/job/coordination child receipts to bind expected topology or node context, operation refs, and pass/deny decisions.
- Add expected-child-ref gates that fail closed on missing, stale, wrong-class, wrong-node, or log-only child evidence.

## Impact

Reviewers gain stronger evidence that the VM check exercised the intended distributed path. Validation remains evidence-only and does not grant authority, policy, provenance, source-gate, resource, or production trust.
