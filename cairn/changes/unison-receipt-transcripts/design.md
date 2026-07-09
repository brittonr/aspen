## Context

Molten has local executable transcript requirements and a deterministic test/replay harness. This change aligns transcript semantics with the artifact registry and evidence ledger: examples should name exact artifacts and compare canonical evidence.

## Design

### Transcript flow

```text
markdown/prose source
  -> canonical transcript stanzas
  -> exact artifact/schema/policy/profile refs
  -> deterministic execution under admitted handlers
  -> canonical trace/receipt oracle comparison
  -> transcript-run-receipt-v1
```

Prose and rendered output remain useful for humans, but canonical stanza records and receipt refs are the normative inputs and outputs.

### Exact refs and profiles

Execution stanzas must bind exact artifact refs or admitted name-resolution receipts. Handler profile, seed/logical time, effect manifest, policy/capability/resource refs, and schema refs must be part of the transcript run key.

### Oracles

Supported expectations should include:

- exact canonical value refs;
- expected receipt kind and decision;
- expected trace markers;
- expected failure class;
- expected diagnostics as non-normative hints.

Terminal text can be rendered from canonical values, but raw logs cannot replace receipt oracles.

### Functional core and shell

Pure cores parse canonical transcript records, validate stanza references, build run keys, and compare receipt oracles. Shells read docs, execute admitted handlers, write receipts, and render documentation.

### Non-goals

- Do not adopt UCM transcript syntax, codebase semantics, typechecker behavior, or hash format.
- Do not let transcript execution observe ambient filesystem, process, network, wall-clock, or environment unless a handler explicitly admits it.
- Do not treat hidden output or rendered logs as pass evidence.
- Do not let transcript examples bypass capability, policy, budget, or provenance gates.