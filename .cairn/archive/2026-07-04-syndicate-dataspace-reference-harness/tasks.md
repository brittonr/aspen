# Tasks: syndicate-dataspace-reference-harness

## Phase 1: Reference adapter and parity core

- [x] [serial] r[molten.syndicate_dataspace.reference_harness] Add a Syndicate-backed local dataspace reference harness over canonical Molten runtime steps and values.
- [x] [serial] r[molten.syndicate_dataspace.parity_receipts] Normalize current Molten and Syndicate harness outcomes into comparable canonical event, assertion, observer, and route refs.
- [x] [parallel] r[molten.syndicate_dataspace.fixture_parity] Add positive and negative parity fixtures for assert, retract, Observe initial delivery, Observe future delivery, cleanup, and denial cases.

## Phase 2: Lifecycle, attenuation, and flow control

- [x] [serial] r[molten.syndicate_dataspace.facet_cleanup] Model actor/session/facet ownership and cleanup in the reference harness with deterministic retraction evidence.
- [x] [serial] r[molten.syndicate_dataspace.cap_attenuation] Map Molten capability/authority decisions to Syndicate cap or rewrite-style attenuation only after Molten admission evidence exists.
- [x] [parallel] r[molten.syndicate_dataspace.flow_control_receipts] Record Syndicate account/debt observations as Molten resource/backpressure evidence without relying on host scheduler timing.

## Phase 3: Trace evidence and docs

- [x] [parallel] r[molten.syndicate_dataspace.trace_evidence] Convert adopted Syndicate trace observations into canonical Molten trace/evidence receipts and mark incomplete traces diagnostic-only.
- [x] [serial] r[molten.syndicate_dataspace.no_wire_compat] Document the Syndicate boundary as semantic/runtime prior art and library implementation detail, not wire, relay, sturdyref, or authority compatibility.
- [x] [serial] r[molten.syndicate_dataspace.reference_harness] r[molten.syndicate_dataspace.parity_receipts] r[molten.syndicate_dataspace.flow_control_receipts] Run focused runtime dataspace, Syndicate harness, resource, and trace evidence tests.
