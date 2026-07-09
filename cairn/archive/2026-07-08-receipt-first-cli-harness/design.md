## Context

`tests/cliharness.rs` and its included parts cover harness runs, replays, gate checks, repro bundles, dogfood receipts, Octet gates, retention flows, node workflows, and more. Many tests already parse Preserves values. The improvement is to make that pattern systematic.

## Design

Introduce a CLI assertion convention:

- every evidence-bearing CLI command writes or prints a canonical artifact;
- tests parse that artifact using the relevant parser;
- assertions inspect decision, artifact kind, subject refs, child refs, diagnostics, and evidence-only caveats;
- stdout/stderr checks prove only human-facing rendering and are never the only pass criterion.

Shared test helpers should remain in the test shell. Pure parsing and decision validation should stay in library modules over in-memory Preserves values. The CLI shell owns process spawning, temporary directories, and output capture.

## Validation

Validation should include positive CLI flows that parse canonical receipts and negative flows for failure artifacts, diagnostic-only bundles, stale refs, malformed reports, and denied gates. Focused CLI harness tests should pass, followed by Cairn validation.
