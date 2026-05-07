{
  id = "runtime-host-oci-lowering-gap",
  display_name = "Runtime host OCI lowering execution E2E gap",
  description = "Metadata-only row: Aspen has OCI lowering admission/receipt model coverage, but no registered suite that ingests an immutable OCI artifact, lowers it into an isolated host, executes it, and reads a secret-safe receipt.",
  layer = "vm",
  owner = "runtime-host-loading",
  runtime_class = "metadata-only",
  runtime_host = {
    kind = "oci-lowering",
    proof_level = "gap",
    support_status = "gap",
    gap_reason = "No suite currently proves OCI artifact ingestion plus lowering into a supported isolated host with execution and secret-safe receipt evidence; raw container execution remains out of scope.",
  },
  prerequisites = [],
  tags = ["runtime-host", "oci", "oci-lowering", "gap", "metadata-only"],
  capabilities = [
    "OCI lowering admission model coverage exists",
    "OCI lowering receipt model coverage exists",
  ],
  isolation_assumptions = [
    "metadata row only; OCI is a packaging/lowering input, not a raw-container production boundary",
    "future E2E must lower into microVM, Hyperlight, WASM, or another supported isolated host",
  ],
  timeout_class = "quick",
  convergence_assertions = [
    "row remains a gap until an OCI artifact is executed through an isolated Aspen runtime host",
  ],
  target = {
    kind = "metadata-only",
  },
}
