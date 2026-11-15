# Aspen bounded CPU render pilot

## Why
Aspen issue #37 needs visible deterministic work on the existing import-free batch executor, without another scheduler or unlicensed demo code.

## What changes
Add an original sphere path tracer, a canonical versioned manifest and assignment protocol, checked assembly, a frozen 640x360 RGB8 scene, executable negative tests and Nix-supported same-host process entrypoints. Keep scene interpretation outside committed-progress infrastructure.

## Non-goals
No native/GPU backend, partial Wasmtime stack restore, profile widening, Hatchet source reuse, cross-host qualification or fabricated execution evidence. Lifecycle fault injection and progress integration stay with the batch/progress owners.
