## Tasks

- [ ] [serial] r[molten.runtime_patterns.sans_io_protocol_core] Inventory Molten protocol/session cores that currently mix transition logic with adapter IO and choose the first implementation slice.
- [ ] [serial] r[molten.runtime_patterns.sans_io_explicit_inputs] Define the explicit state, event, deterministic freshness, limit, authority, policy, replay, and effect-response inputs required by the selected core.
- [ ] [parallel] r[molten.runtime_patterns.sans_io_transition_outputs] Define transition outputs for state deltas, outbound envelopes, effect intents, diagnostics, and receipt input facts without shell side effects.
- [ ] [parallel] r[molten.runtime_patterns.sans_io_shell_adapter] Refactor the selected shell so Iroh, Redb, dataspace, receipt, tracing, and adapter effects occur only after the pure core result and normal admission gates.
- [ ] [parallel] r[molten.runtime_patterns.sans_io_replay_binding] Bind protocol inputs, outputs, before/after state refs, and gate receipts into replay evidence for the selected slice.
- [ ] [serial] r[molten.testing.sans_io_positive_negative_fixtures] Add positive tests for deterministic transitions and negative tests for malformed messages, missing evidence, illegal transitions, hidden ambient inputs, and pre-admission shell mutation.
- [ ] [serial] r[molten.testing.sans_io_positive_negative_fixtures] Update developer docs and run focused protocol tests plus Cairn validation.