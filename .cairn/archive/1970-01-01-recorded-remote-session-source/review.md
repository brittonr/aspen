# Review: capability-authenticated recorded remote sessions

Reviewed by the integration owner against the proposal, design and `remote-dataspace` delta.

- Receiver ownership: one immediate-durability Redb transaction serializes generation allocation and every committed observation; generations never reset and lost/repaired databases refuse rather than reinitialize (`lost_database_never_reinitializes_an_acknowledged_generation`).
- Sender knowledge: every build replays the verified hash chain under a fresh read transaction; no snapshot or reference-list import constructor exists. Stale, closed, gapped and rebound sources refuse with canonical evidence.
- Cutover: envelope, protocol and job builders and local publication consume the live view; the unbound live gossip entrypoint refuses referenced envelopes (`introduction_is_not_a_live_receiver_acknowledgment`). Wire shape is unchanged.
- CLI: `test remote session admit|commit|close|receive` and the sender/receiver/generation source flags are exercised by the real-binary CLI test.
- Every spec scenario maps to an executed test listed in `evidence/2026-10-30-verification.md`.

No network admission, authority grant or remote trust is claimed. Accepted for archive.
