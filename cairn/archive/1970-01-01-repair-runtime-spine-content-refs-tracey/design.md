# Design: Repair runtime-spine canonical content-ref Tracey links

## Success contract

A requirement leaves inherited debt only when current production logic and a focused positive or negative test support its complete accepted wording.
Completion requires ten exact repairs, two explicit rejections, deterministic typed evidence, zero dangling markers, and repository validation.

False completion includes promoting a shared parser into proof that all construction uses shared helpers, treating archived task completion as current evidence, or treating content identity as trust.

## Search registry

### Shared parser and typed identity

Review `ContentRef`, canonical shape validation, byte/hash/hex helpers, canonical Preserves hashing, serialization, and malformed-shape tests.

### Materialized storage and filename readback

Review ledger, chunk-store, ingress, and evidence readback paths.
Require validated filename conversion, local existence checks, and byte or canonical-value ref recomputation.

### Node-control and trust separation

Review request, payload, envelope, transport receipt, and subreceipt parsing.
Require valid-shaped requests without authority or resource evidence to deny.

### Runtime and migration surfaces

Review runtime values, messages, assertions, observations, events, turn receipts, snapshots, and the listed migrated subsystem validators.
Require canonical Preserves identity and replay-stable focused tests.

### Adversarial formatting audit

Search production source for subsystem-local `blake3:` formatting and prefix manipulation.
Any current counterexample keeps broad helper-only or no-ad-hoc claims in inherited debt.

## Evidence manifest

A typed Nickel manifest records each candidate, source area, implementation path, verification path, evidence scope, and rejection reason.
Generated JSON is a deterministic validation input.

## Freshness gate

The inherited Tracey Nix check exports the manifest, compares generated JSON, checks exact counts and unique identifiers, verifies declared markers, and enforces accepted and rejected baseline states.

## Functional boundary

The patch adds marker comments, evidence metadata, and focused parser/readback assertions.
Production behavior remains unchanged.

## Non-claims

The repair does not prove universal helper-only construction, removal of all ad hoc formatting, content-ref trust, complete runtime-spine coverage, release readiness, or whole-system correctness.
