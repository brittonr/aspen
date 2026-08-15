## Context

Accepted testing-harness requirements already mention property counterexamples and counterexample shrinking. The current improvement packages the behavior into concrete artifacts and promotion rules.

## Design

Define a `hegel-counterexample-fixture` artifact that binds:

- property id and requirement ids;
- generator profile ref;
- generation seed;
- shrink path;
- final shrunk Preserves input;
- replay identity;
- expected failure or fixed regression oracle;
- trace and receipt refs when available;
- confidentiality metadata.

The pure core should build and validate fixture metadata from typed inputs. The shell owns integration with Hegel test execution, file output, and developer commands.

Promotion to regression should require review metadata: source property, old failure refs, new deterministic suite entry, reason class, and status after the bug is fixed or accepted as known-deny behavior.

## Validation

Positive tests should validate a complete fixture and promotion record. Negative tests should reject missing seed, missing shrink path, stale replay identity, malformed Preserves input, missing confidentiality metadata for sensitive inputs, and regression promotion without review evidence.
