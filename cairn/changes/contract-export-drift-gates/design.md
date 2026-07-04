# Design: Contract export drift gates

## Context

Contract source, generated artifacts, Preserves boundary metadata, and Rust parsers form one evidence chain. Any link can drift unless the gate checks them together.

## Gate phases

1. Evaluate positive Nickel fixtures and compare the generated value to checked-in JSON or Preserves artifacts.
2. Evaluate negative Nickel fixtures and require failure.
3. Parse checked-in Preserves artifacts through existing Rust admission parsers.
4. Check Preserves boundary schema metadata, record labels, and arity against the artifacts used by Rust.
5. Report deterministic diagnostics that identify stale generation, schema mismatch, parser rejection, or unexpected negative acceptance.

## Implementation boundary

The pure core should compare in-memory normalized values and expected metadata. The imperative shell may read files, invoke Nickel export tooling, and call Rust tests or CLI checks.

## Scope

Start with plugin extension contracts/grants, production profiles, multinode scenarios, peer profiles, and Cairn policy JSON. Expand to future contract exports as they are introduced.
