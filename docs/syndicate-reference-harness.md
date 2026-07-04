# Syndicate reference harness boundary

r[impl molten.syndicate_dataspace.no_wire_compat] r[verify molten.syndicate_dataspace.no_wire_compat]

Molten uses Syndicate only as a local reference-semantics harness for adopted dataspace scenarios.

Normative Molten boundaries remain canonical Preserves runtime steps, BLAKE3 value refs, policy gates, authority gates, resource gates, provenance/source-gate evidence, and Molten receipts. Syndicate bag/account/trace/capability observations are recorded as Molten evidence for parity, diagnostics, and backpressure review, but they do not grant authority, transport, policy, resource, provenance, source-gate, retention, plugin, or execution trust.

The harness does not claim Syndicate wire-protocol, relay, sturdyref, service, trace, capability, or authority compatibility. Any future compatibility surface needs its own scoped change with explicit requirements, pass/deny fixtures, and receipt evidence.
