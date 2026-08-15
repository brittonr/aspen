## Why

Content-addressed artifacts are hard to operate without human-readable names, but names become dangerous when they are treated as identity or authority. Unison's namespace model is useful prior art: names are pointers to immutable hashes and may change without rewriting definitions.

Molten should adapt this as explicit, receipt-backed name view records over artifact refs. A name may help humans and tools find artifacts, but exact artifact refs remain the only dependency and execution identity.

## What Changes

- Add canonical name, alias, tag, and channel view records that point to exact artifact refs or artifact sets.
- Require dependency declarations, transcript expectations, remote execution requests, and policy admissions to pin exact refs after name resolution.
- Emit ambiguity and stale-name diagnostics instead of silently selecting one artifact.
- Keep name views non-authoritative: they do not grant capability, provenance, policy trust, source-gate trust, retention, or execution rights.

## Impact

- **Files**: artifact registry metadata, catalog/search display, transcript parsing, remote execution envelopes, diagnostics.
- **Testing**: positive fixtures for deterministic name resolution and exact ref pinning; negative fixtures for ambiguous names, stale channels, unauthorized pointer updates, and name-only execution.
- **Security**: names remain discovery metadata. Authority and trust are carried by separate evidence gates.