# Design: Classify inherited Tracey debt

## Functional core

The classifier accepts a sorted baseline and accepted requirement definitions.
It returns sorted rows or explicit validation issues.

Each row contains:

- accepted specification path;
- source area from the requirement identifier;
- conservative class;
- requirement identifier.

The core assigns `accepted-implementation-unestablished` unless reviewed evidence establishes a stronger class.
The first inventory contains no inferred replacement, obsolescence, invalidity, or implementation claims.

## Imperative shell

The shell reads `cairn/specs`, reads the exact baseline, writes tab-separated output, and prints bounded counts.
It performs no repository mutation other than its declared output file.

## Direct repair rule

A baseline entry can be removed only when existing production logic and tests directly satisfy the accepted requirement.
The change repairs three entries that meet this rule:

- ChoRus remains a design reference only;
- the Valence stack adapter remains evidence-only;
- receipt-driven traceability derives coverage from canonical receipt fields.

## Validation

The Nix check compiles and tests both Tracey tools.
It reproduces the classification inventory byte-for-byte.
It validates typed Nickel metadata and checks BLAKE3 identities.
Negative tests reject missing definitions, duplicate definitions, malformed baselines, and foreign namespaces.

## Non-claims

The inventory is review routing data.
It does not establish implementation, behavioral correctness, lifecycle replacement, obsolescence, invalidity, or release readiness.
