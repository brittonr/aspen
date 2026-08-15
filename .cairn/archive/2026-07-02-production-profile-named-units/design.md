# Design: Production profile named units

## Context

Numeric resource limits need unit and intent. Nickel can express both through named constants while still exporting plain JSON numbers for Rust and receipt code.

## Unit constants

Add a small set of reviewed constants for byte units, time units, and profile thresholds. Threshold names should describe the policy meaning, not only the arithmetic. Examples include maximum queue depth, maximum receipt size, maximum store size, maximum delivery latency, and maximum recovery time.

## Export stability

The concrete production profile should use the named constants. `nickel export` must produce the same JSON numeric values for the existing pilot profile unless a threshold is intentionally changed in the same review.

## Review behavior

Changing a threshold becomes a named policy diff. The change should update the constant name or value, the operator documentation if needed, and the fixture expectations that assert export stability.

## Boundaries

Named units document and constrain the static profile. They do not measure live resource use or guarantee capacity; runtime observability and resource-pressure receipts remain responsible for live evaluation.
