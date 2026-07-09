# Replay coverage readiness

Replay coverage matrices summarize supplied replay evidence by subsystem and workflow. They are canonical readback artifacts for operators, not substitutes for individual replay receipts, release gates, authority, policy, source-gate, provenance, resource, retention, transport, or execution checks.

Each row records a subsystem, workflow, eligibility class, fresh-run ref, replay-verify ref, second-fresh-run ref, negative/tamper evidence ref, optional replay-index ref, and caveat refs. Deterministic and recorded rows require positive and negative evidence. Diagnostic-only and non-replayable rows must carry caveats and cannot satisfy deterministic replay evidence.

The pure matrix core validates unique rows, required evidence, stale or malformed refs, and diagnostic-only exclusions, then returns a pass/deny matrix with canonical diagnostics and evidence-only checks.
