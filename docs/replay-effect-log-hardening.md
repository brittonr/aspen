# Replay effect-log hardening

Replay effect logs are validated as ordered evidence before deterministic replay accepts recorded responses. The validator is a pure core: callers provide parsed effect entries, consumed effect observations, the expected run identity ref, and the expected handler profile ref. It returns canonical validation evidence with a stable validation ref and denial diagnostics.

The boundary rejects gaps, duplicate sequences, duplicate request refs, missing recorded entries, unused recorded entries, request/response/effect-kind/boundary mismatches, stale run identity refs, stale handler profile refs, malformed effect kind tokens, and any consumed observation that reports live effect fallback. The evidence is replay-only; it does not grant authority, resource rights, provenance trust, or permission to issue external effects.

`verify_fixture_value` and `verify_fixture_record_value` invoke the effect-log validator before comparing downstream trace or final-state refs, keeping malformed effect logs from being reported as later state drift.
