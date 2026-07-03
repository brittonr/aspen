# Design: verification run receipts

## Scope

This change makes test and validation executions first-class evidence. It does not make command logs normative and does not bypass existing harness, replay, release, or subsystem gates.

## Receipt model

A verification run receipt should be a canonical Preserves artifact with fields for schema, decision, requirement id, coverage kind, target, normalized argv, environment profile ref, toolchain or Nix input refs, exit status, stdout ref, stderr ref, produced artifact refs, and checks.

The pure core validates the receipt shape and recomputes the decision from explicit inputs. The CLI shell runs commands, writes captured artifacts, computes BLAKE3 refs, and calls the pure core to render the receipt.

## Traceability integration

Traceability entries may continue to accept compatibility coverage strings, but receipt-backed entries become the preferred proof. A passing entry requires the receipt requirement id and positive/negative kind to match the traceability entry, the command target to exist, the receipt decision to match the kind's expected result, and every named artifact ref to validate.

## Hegel RS properties

Hegel RS generated cases should cover:

- same receipt inputs produce the same canonical ref;
- changing argv, target, kind, requirement id, exit status, or artifact refs changes the receipt ref or denies validation;
- stale or malformed artifact refs never produce a passing traceability entry;
- deny receipts remain evidence only and do not satisfy positive coverage.

## Non-goals

- No general-purpose build system.
- No trust in rendered stdout/stderr text.
- No implicit authority from a passing verification receipt.
