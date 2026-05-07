{
  id = "runtime-host-wasm-gap",
  display_name = "Runtime host WASM runner E2E gap",
  description = "Metadata-only row: Aspen has WASM admission/lifecycle model coverage and plugin plumbing, but no registered suite that boots Aspen and executes a WASM component through the real runtime path.",
  layer = "vm",
  owner = "runtime-host-loading",
  runtime_class = "metadata-only",
  runtime_host = {
    kind = "wasm",
    proof_level = "gap",
    support_status = "gap",
    gap_reason = "No suite currently starts Aspen with the real WASM runtime path, invokes a component through product RPC/host-call flow, and asserts product-visible output or receipt evidence.",
  },
  prerequisites = [],
  tags = ["runtime-host", "wasm", "gap", "metadata-only"],
  capabilities = [
    "WASM admission/lifecycle model coverage exists",
    "WASM plugin install/reload plumbing exists",
  ],
  isolation_assumptions = [
    "metadata row only; does not execute a WASM module",
    "do not count plugin CLI install/reload as runtime execution proof",
  ],
  timeout_class = "quick",
  convergence_assertions = [
    "row remains a gap until a real Aspen-spawned WASM execution suite exists",
  ],
  target = {
    kind = "metadata-only",
  },
}
