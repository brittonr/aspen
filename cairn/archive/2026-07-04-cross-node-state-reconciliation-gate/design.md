# Design: cross-node state reconciliation gate

The reconciliation core accepts explicit node evidence summaries, a topology ref, scenario fixture ref, expected equivalence classes, allowed variance refs, and receipt refs. It returns pass or deny diagnostics plus a canonical reconciliation receipt.

Inputs are grouped by concern:

- node identity and topology refs;
- startup, health, shutdown, and control-loop refs;
- ingress, queue, dispatch, reconcile, ack, and protocol refs;
- ledger, coordination, job, chunk manifest, and receipt-index refs;
- expected equality sets and declared variance sets.

The core never reads node directories or logs. The shell collects per-node artifacts, computes refs, and passes summaries to the core. Logs are attached as diagnostics only.

The first gate should prove that successful cross-node workflows converge on the expected protocol and ack evidence while rejecting stale refs, wrong topology, missing node evidence, duplicate semantic commit evidence, divergent queue refs without variance, and log-only reconciliation.
