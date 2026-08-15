## Why

Hegel property tests are already present and valuable, but a property failure is most useful when its shrunk counterexample becomes an ordinary deterministic regression case. Without a canonical fixture and replay identity, counterexamples can remain trapped in transient test output.

This change makes property failures first-class harness evidence: generated input, shrink path, final shrunk Preserves fixture, and replay command are preserved so the case can be replayed without invoking the generator.

## What Changes

- Define canonical Hegel counterexample fixture artifacts.
- Record generation seed, shrink path, shrunk input, runtime identity, trace refs, and diagnostics.
- Add promotion flow from counterexample fixture to deterministic regression suite entry.
- Redact or encrypt sensitive generated inputs before export.

## Impact

Property testing becomes a source of durable regression tests rather than only randomized confidence.
