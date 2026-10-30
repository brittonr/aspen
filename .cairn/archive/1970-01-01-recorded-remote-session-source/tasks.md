# Tasks

- [x] [serial] Implement receiver-owned transactional local recording with monotonic generations and bounded hash-chained observations. r[molten.remote_dataspace.recorded_session_source]
      `src/remote/parts/dataspace/p010/recorded.rs` and `recordedstore.rs`: one immediate-durability Redb transaction per
      allocation/commit/close/receive; bounds 1,024 keys, 4,096 observations and 4 MiB per session with reserved close
      capacity; lost or repaired databases refuse instead of reinitializing a generation.
- [x] [serial] Derive sender knowledge from validated current committed history; refuse unknown/stale/closed generation builds with canonical evidence. r[molten.remote_dataspace.recorded_session_source]
      `RecordedSessionStore::sender_view` replays the verified hash chain under a fresh read transaction on every build;
      `remote-dataspace-envelope-build-denial-v1` names every unknown semantic reference.
- [x] [serial] Cut over envelope builders and receiver semantic reference checks without wire or authority upgrade. r[molten.remote_dataspace.recorded_session_source]
      `build_envelope_with_evidence`, protocol and job-worker builders and local publication take the live view
      (`p012/body.rs`); unbound live gossip publication refuses referenced envelopes (`p013/body.rs`). Envelope wire shape
      is unchanged.
- [x] [serial] Expose actual CLI open/commit/close/build hooks and positive/negative runnable fixtures. r[molten.remote_dataspace.recorded_session_source]
      `molten test remote session admit|commit|close|receive` (`src/cli/workflow/remote/session.rs`) and the
      `--sender-state-root`/`--receiver-state-root`/`--session-generation` source flags on envelope build, publish-local
      and job worker commands; operator walkthrough in `docs/handbook/job-workflows/preparing-a-recorded-worker-exchange.md`.
- [x] [serial] Run scoped and required integration verification after concurrent implementation lands; record only executed evidence. r[molten.remote_dataspace.recorded_session_source]
      See `evidence/2026-10-30-verification.md`.
