# Design: vm-failure-repro-bundles

## Overview

Add VM-aware failure repro export to the evidence shell while reusing the existing pure bundle verification and pass-gate logic. The exporter runs after a shard or aggregate produces deny, unavailable, validation failure, or host-support failure evidence.

## Bundle contents

A VM failure repro bundle contains refs for:

- scenario fixture and metadata;
- topology and topology profile;
- scheduler or VM shard plan when present;
- seed or fault plan refs when present;
- per-node evidence and node summaries;
- child workflow receipts;
- validation and gate receipts;
- diagnostic log refs marked diagnostic-only;
- redaction policy refs, privacy markers, reveal receipt refs when needed;
- replay status and caveats.

## Replay classification

Deterministic simulation bundles may be replayable. Local multiprocess bundles can be replayable only if command, process plan, and effect records are complete. VM/live bundles are non-replayable diagnostic evidence unless a separate recorded effect log exists.

## Validation

Verification recomputes bundle and payload refs, checks seal metadata, validates privacy and redaction requirements, and rejects diagnostic-only bundles as pass evidence. Any private attachment requires an exact reveal receipt before materialization.

## Boundaries

Failure repro bundles never satisfy pass gates. They are triage artifacts only and cannot override canonical deny receipts or unavailable host-support evidence.
