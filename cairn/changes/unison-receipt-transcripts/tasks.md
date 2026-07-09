## Tasks

- [ ] [serial] r[molten.transcripts.exact_artifact_ref_expectations] Require execution stanzas to bind exact artifact refs or admitted name-resolution receipts plus schema, policy, effect manifest, handler profile, capability, and resource refs.
- [ ] [serial] r[molten.transcripts.canonical_receipt_oracles] Represent transcript expectations as canonical value, trace, receipt-kind, decision, and failure-class oracles rather than raw terminal text.
- [ ] [parallel] r[molten.transcripts.diagnostic_output_non_normative] Keep stdout, stderr, logs, prose, hidden output, and rendered markdown diagnostic-only unless explicitly canonicalized as Preserves values.
- [ ] [parallel] r[molten.transcripts.handler_profile_seed_binding] Bind handler profile refs, seeds, logical time, and effect manifest refs into transcript run keys, replay receipts, and evaluation-cache keys.
- [ ] [serial] r[molten.transcripts.receipt_transcript_validation] Add positive and negative fixtures for deterministic replay, expected failures, stale refs, profile mismatch, nondeterministic output, missing capabilities, hidden output, and UCM compatibility denial.