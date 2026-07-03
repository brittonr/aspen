# Design: proof obligation decomposition

## Scope

This change standardizes how Molten decomposes proof evidence. It does not require every tiny unit test to produce a separate receipt; it applies when a workflow or release claim spans multiple semantic obligations.

## Obligation model

A proof obligation should name a stable id, claim class, subject ref, prerequisite refs, produced receipt refs, and evidence-only caveats. Supported initial classes are input-validation, canonicalization, admission, mutation-boundary, replay-determinism, and fail-closed-negative.

An aggregate proof manifest is passing only when all required child obligations are present, bound to the same subject or declared subsubject, and have passing receipts for positive claims or expected-deny receipts for negative claims.

## Functional core boundary

The pure core constructs and validates obligation manifests from in-memory DTOs. The CLI/report shell discovers files and command receipts, but cannot decide pass without the core recomputing the obligation graph.

## Hegel RS properties

Generated obligation graphs should verify that deterministic sorting yields stable refs, missing required children deny, duplicated child ids deny, mismatched subjects deny, and negative obligations cannot be substituted for positive obligations.

## Non-goals

- No new authority model.
- No replacement for subsystem-specific gates.
- No hidden dependency on rendered summaries.
