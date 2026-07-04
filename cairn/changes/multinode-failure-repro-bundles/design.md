# Design: multinode failure repro bundles

A repro bundle is a sealed evidence artifact, not automatic pass evidence. The bundle contains canonical refs for the scenario fixture, topology, scheduler, seed, fault plan, commands, node evidence summaries, run receipt, reconciliation receipt when present, diagnostics, logs, redaction policy, and replay status.

Deterministic simulation bundles must support replay from stored inputs. Local multiprocess and VM bundles may be marked non-replayable when process scheduling, platform support, or live transport observations cannot be reproduced exactly; they must still verify seal integrity and receipt bindings.

The pure verifier recomputes embedded refs, checks seal metadata, validates redaction coverage, verifies receipt bindings, and classifies replay support. The shell owns reading bundle files, unpacking verified contents, and materializing redacted logs.

Negative fixtures cover tampered topology, tampered receipt, missing redaction transform, missing private reveal, stale scenario fixture, unsealed legacy bundle, and diagnostic-only failure bundle being used as pass evidence.
