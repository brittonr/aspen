## Why

Unison transcripts are useful because examples and bug reports are executable documentation. Molten should adapt the executable-doc pattern around canonical Preserves stanzas, exact artifact refs, deterministic handler profiles, and receipt oracles.

Rendered output and prose are helpful, but normative pass evidence must come from canonical traces and receipts. This keeps examples reproducible and safe enough for policy/release gates.

## What Changes

- Require transcript stanzas that affect execution to bind exact artifact refs, schema refs, handler profile refs, seeds, policy refs, and capability/resource evidence.
- Treat expected results as canonical receipt/trace oracles rather than raw terminal text.
- Keep rendered stdout/stderr/logs diagnostic-only unless explicitly canonicalized as Preserves values.
- Add positive and negative fixtures for passing transcripts, expected failures, stale artifact refs, wrong handler profile, nondeterministic output, hidden output, and UCM compatibility claims.

## Impact

- **Files**: local executable transcripts, test harness, evaluation cache, catalog rendering, docs.
- **Testing**: positive fixtures for deterministic transcript replay; negative fixtures for stale refs, log-only expectations, seed/profile mismatch, missing capabilities, and unsupported syntax.
- **Security**: transcripts do not grant authority. They run only through admitted handlers, capabilities, policy, budgets, and source/provenance gates.