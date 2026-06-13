## Why

Molten plans to run native, Wasm, and Steel actors behind deny-by-default adapters, but it does not yet define a uniform way for executable artifacts to declare the effects they need or for tests to swap production adapters for local, simulated, or chaos handlers.

Unison abilities are useful prior art: programs declare the effects they require, and handlers provide the interpretation. Molten should adapt this as effect/capability manifests and admitted handler bindings, not as a new source language or unrestricted algebraic-effects runtime.

## What Changes

- Add effect/capability manifests for executable Molten artifacts.
- Require Wasm, Steel, native, choreography, and job artifacts to declare needed effects before admission.
- Model handlers as adapter bindings that interpret declared effects through Molten runtime APIs and policy gates.
- Support production handlers for Iroh, Redb, dataspace, Wasmtime hostcalls, time, randomness, and external services.
- Support local, mock, tracing, profiling, and chaos handlers for deterministic testing of distributed programs.
- Gate handler binding through Basalt capabilities, Nickel static contracts, reviewed Steel dynamic predicates where needed, Trellis predicates, and Cairn receipts.
- Keep effect declarations in artifact metadata so semantic search and remote execution can reject missing authority early.

## Impact

This turns Molten's deny-by-default adapter policy into a developer-visible contract. Programs can be tested with local handlers, profiled with observability handlers, and deployed with production handlers while preserving the same artifact identity and declared effect surface.
