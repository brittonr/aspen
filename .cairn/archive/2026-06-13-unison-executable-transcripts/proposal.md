## Why

Molten needs documentation, examples, bug reproductions, and integration tests that exercise the runtime through the same CLI/API surfaces developers use. Ordinary prose and ad hoc shell scripts are too easy to drift away from the actual runtime behavior.

Unison transcripts provide a useful pattern: markdown documents with fenced interaction stanzas that run against a fresh codebase and produce checked output. Molten should adopt this as executable transcripts over canonical runtime state, handler profiles, and receipt/trace comparison.

## What Changes

- Add executable transcript artifacts for Molten docs, examples, bug reports, and regression tests.
- Define markdown stanza types for Molten CLI commands, config snippets, Preserves envelopes, policy fixtures, actor code artifacts, and expected results.
- Run transcripts against fresh deterministic local state by default, with optional fork/in-place modes gated by policy.
- Support expected failure, known bug, hidden/noisy output, and stateful edit/load-style stanzas.
- Capture canonical output as trace and receipt artifacts, not only text snapshots.
- Allow transcripts to pin artifact ids, dependency closures, handler profiles, seeds, and policies.
- Integrate transcript results with the evaluation cache and artifact registry.

## Impact

Molten documentation can become reproducible tests and bug reports can become permanent regression artifacts. The first milestone can implement a small transcript runner for CLI-like commands against an in-process local runtime and compare expected canonical traces.
