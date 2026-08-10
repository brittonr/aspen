# Repair three-node membership traceability

## Why

Tracey reports `molten.testing.multinode.three_node_membership_negatives` as the repository's only dangling requirement marker.
The accepted testing-harness specification contains the successor requirement `molten.testing.multinode.three_node_vm_membership_negatives`, but Tracey reports it as missing.

The implementation and negative test already enforce the successor requirement's role, quorum, authority, and log-only denial classes.
The marker identity did not move when the accepted requirement identity changed.

## What changes

- Retarget the obsolete implementation and verification markers to the accepted successor requirement.
- Keep the existing negative behavior and accepted specification unchanged.
- Record before-and-after Tracey evidence and rerun the focused denial test.

## Impact

This change repairs traceability identity only.
It does not add behavior, broaden VM claims, change requirement text, or close unrelated repository-wide coverage debt.
