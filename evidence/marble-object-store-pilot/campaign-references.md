# ChaosControl campaign references — marble object-store pilot

The pilot's storage-path faults trace to the 2026-09-08 logical audit
([`docs/audits/logical-bugs-2026-09-08.md`](../../docs/audits/logical-bugs-2026-09-08.md)),
whose F-series campaigns repeatedly surfaced delivery, shutdown, and
replication faults rooted in physical storage behavior. The spike records the
following references; ChaosControl campaign evidence itself stays external and
is not claimed by this repository.

## F-series fault set referenced by the spike

| Finding | Class | Spike relevance |
|---|---|---|
| F01 denied shutdown changes node lifecycle state | shutdown admission | batch commit ordering behind the storage seam |
| F03 dedup commit precedes unrecoverable ingress enqueue | durable ingress | write-cache visibility until `write_batch` returns |
| F04 historical success suppresses repair after replica loss | replication | recovery replays only atomically recovered batches |
| F05 cleanup removes the only replica in a required domain | replication | maintenance keeps live objects while rewriting files |
| F07 missing targets disappear from under-replication status | replication | absent digest fails lookup explicitly |
| F08 duplicate claim returns a later consumer token | delivery | identical digests resolve to one logical object |
| F12 exponential retry wraps to zero | delivery arithmetic | checked, bounded counters in measurement harness |
| F14 historical shutdown success becomes current observation | shutdown | recovery receipt records only recovered mappings |

## Deterministic in-repo stand-ins

The separate-process fixture
`marble_store::tests::multiprocess::crash_mid_flight_replays_only_atomically_recovered_batches`
kills a writer mid-flight and verifies the crash contract deterministically.
It is not a ChaosControl campaign receipt and does not become one by passing.

## Non-claims

These references scope the spike to the recorded measurements. They do not
authorize a default-path change, do not transfer ChaosControl evidence into
this repository, and do not prove runtime, replication, or delivery
correctness.
