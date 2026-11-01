## Context

`molten.wasm.component.v1` admits an exact engine cohort: the profile document names Wasmtime
and Wasmtime-WASI versions, admission rejects any other toolchain, and the runtime source-cohort
record binds the shared crate's published revision. Molten therefore moves four things
together: the Cargo pins, the shared crate revision (with its own published evidence), the
admitted profile documents and their generated exports, and the provenance record that names
the shared cohort revision.

## Decisions

### Decision: 49.0.5 through the shared crate's published revision

**Choice:** Move to `wasmtime 49.0.5` and `hardened-wasmtime-runtime` at
`57c834dfd0da2f092e2cbbd0e48bb60b61e2d06e` instead of pinning 48.0.8 or 50.x locally.

**Rationale:** 49.0.5 is the lowest published release inside every advisory's patched range
(48.0.8 still fails -0327, -0359, -0362, -0363 and -0379). The shared crate owns the engine
configuration and already pins the same exact version, so consuming its patched revision keeps
one engine identity across Molten and the crate rather than allowing two.

### Decision: keep the wRPC alias at its own engine

**Choice:** Leave `wasmtime-rpc = "=33.0.0"` untouched.

**Rationale:** That alias belongs to the optional wRPC transport's own engine cohort, not to
Molten's admitted component profile. Moving it would change the wRPC rail's identity without
its own review; the advisory position of that optional, feature-gated rail is recorded as a
separate blocker in `pilot-wrpc-component-transport`.

## Risks / Trade-offs

- Wasmtime 49 rejects the legacy `(memory $i "name")` component syntax; parity fixtures moved to
  `(core memory $i "name")`/`(core func $i "name")`. This is a fixture-only change, but it is a
  syntax requirement, not a behavioral claim.
- The engine move changes every receipt's toolchain identity. Historical receipts remain
  historical; fresh startup and component receipts name 49.0.5.
