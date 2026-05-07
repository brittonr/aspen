{
  id = "runtime-host-hermit-gap",
  display_name = "Runtime host Hermit/unikernel E2E gap",
  description = "Metadata-only row: Aspen has Hermit/unikernel profile/admission/receipt model coverage, but no registered suite that spawns and observes a real Hermit/unikernel workload through Aspen.",
  layer = "vm",
  owner = "runtime-host-loading",
  runtime_class = "metadata-only",
  runtime_host = {
    kind = "hermit",
    proof_level = "gap",
    support_status = "gap",
    gap_reason = "No suite currently launches a Hermit/unikernel workload through Aspen's runtime path and records bounded lifecycle/output/receipt evidence.",
  },
  prerequisites = [],
  tags = ["runtime-host", "hermit", "unikernel", "gap", "metadata-only"],
  capabilities = [
    "Hermit/unikernel profile/admission model coverage exists",
    "Hermit/unikernel receipt model coverage exists",
  ],
  isolation_assumptions = [
    "metadata row only; does not launch a unikernel",
    "future E2E must use a real Aspen-spawned VM-backed guest or equivalent supported runner path",
  ],
  timeout_class = "quick",
  convergence_assertions = [
    "row remains a gap until a real Aspen-spawned Hermit/unikernel suite exists",
  ],
  target = {
    kind = "metadata-only",
  },
}
