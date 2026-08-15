# Design: generated distributed fault interleavings

Use Hegel at the pure simulator boundary. Generators create bounded topologies, scheduler profiles, deterministic seeds, command sequences, and fault plans within the simulator's declared limits. The property core calls `run_distributed_simulation` with explicit values and asserts invariants over returned receipts, events, committed operations, denied operations, diagnostics, and final-state refs.

Initial invariants:

- deterministic replay returns stable run and final-state refs;
- duplicate delivery does not create a second semantic commit;
- restart and crash replay do not drop already admitted evidence;
- unauthorized transport, stale evidence, corrupted receipts, ambient drift, resource pressure, and partitioned quorum deny before side effects;
- mutation of topology, payload, operation id, schedule, or required evidence changes canonical evidence or fails closed.

When a generated case fails, the shell writes a repro bundle with seed, topology, scheduler profile, fault plan, command set, expected invariant, observed diagnostics, and receipt refs. The bundle is evidence for debugging unless a gate separately validates it as pass or deny evidence.
