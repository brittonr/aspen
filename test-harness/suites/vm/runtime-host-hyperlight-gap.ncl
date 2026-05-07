{
  id = "runtime-host-hyperlight-gap",
  display_name = "Runtime host Hyperlight E2E gap",
  description = "Metadata-only row: Aspen has Hyperlight profile/admission/receipt model coverage and build plumbing, but no registered real Aspen-spawned Hyperlight execution suite.",
  layer = "vm",
  owner = "runtime-host-loading",
  runtime_class = "metadata-only",
  runtime_host = {
    kind = "hyperlight",
    proof_level = "gap",
    support_status = "gap",
    gap_reason = "No suite currently launches a Hyperlight workload through Aspen's runtime path and records product-visible lifecycle/output/receipt evidence.",
  },
  prerequisites = [],
  tags = ["runtime-host", "hyperlight", "gap", "metadata-only"],
  capabilities = [
    "Hyperlight profile/admission model coverage exists",
    "Hyperlight receipt model coverage exists",
  ],
  isolation_assumptions = [
    "metadata row only; does not launch Hyperlight",
    "future E2E may require hardware/runtime support and explicit gated prerequisites",
  ],
  timeout_class = "quick",
  convergence_assertions = [
    "row remains a gap until a real Aspen-spawned Hyperlight suite exists",
  ],
  target = {
    kind = "metadata-only",
  },
}
