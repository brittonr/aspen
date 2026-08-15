# Design: multinode declarative scenario fixtures

Keep fixture interpretation as a pure core over explicit input values. Nickel remains the authoring and type-checking layer; Rust receives exported fixture data or in-memory test values and validates schema, profile wiring, topology refs, expected artifact kinds, unavailable policy, and variance declarations without reading files or environment state.

A scenario fixture declares:

- scenario id, purpose, and evidence scope;
- topology role map, node identities, and allowed links;
- profile id, command surface, cost class, and release-review status;
- deterministic seed, fault-plan ref, expected receipt kinds, and diagnostic log refs;
- unavailable handling and explicit caveats for evidence-only claims.

The imperative shell owns Nickel export, VM command invocation, local process spawning, and writing evidence artifacts. The shell must not infer missing fixture values from ambient runtime state. Missing fixture values deny before a pass receipt is minted.

Validation should include a readback helper that renders the canonical fixture ref and the derived metadata ref. Reviewers should be able to compare fixture refs across CI runs and see when topology, commands, expected artifacts, or variance declarations changed.
