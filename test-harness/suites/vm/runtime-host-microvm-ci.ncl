{
  id = "runtime-host-microvm-ci-vm",
  display_name = "Runtime host microVM CI worker E2E",
  description = "Aspen-spawned CloudHypervisorWorker path: start Aspen, create a CI VM golden snapshot, submit ci_vm jobs through aspen-cli, and observe guest-backed completion.",
  layer = "vm",
  owner = "runtime-host-loading",
  runtime_class = "nixos-vm",
  runtime_host = {
    kind = "microvm",
    proof_level = "aspen-spawned-execution",
    support_status = "e2e-registered",
  },
  prerequisites = ["nix-command", "nested-kvm"],
  tags = ["runtime-host", "microvm", "cloud-hypervisor", "ci-vm", "aspen-spawned", "expensive", "impure"],
  capabilities = [
    "CloudHypervisorWorker",
    "ci_vm job execution",
    "golden snapshot restore",
    "Aspen-spawned microVM host proof",
  ],
  isolation_assumptions = [
    "requires nested KVM and Cloud Hypervisor support",
    "requires sandbox-disabled or impure Nix build mode for VM execution",
    "does not prove WASM, OCI lowering, Hyperlight, or Hermit runner execution",
  ],
  timeout_class = "soak",
  expected_artifacts = [
    {
      path = "/tmp/aspen-node.log",
      kind = "log",
    },
    {
      path = "/tmp/aspen-ci-vms/snapshots/golden/memory",
      kind = "snapshot",
    },
  ],
  convergence_assertions = [
    "aspen-node reaches cluster leader state",
    "CloudHypervisorWorker creates a golden snapshot",
    "ci_vm jobs submitted through aspen-cli reach completed state",
  ],
  target = {
    kind = "nix-build",
    flake_attr = "checks.x86_64-linux.vm-snapshot-e2e-test",
    check_attr = "vm-snapshot-e2e-test",
    nix_file = "nix/tests/vm-snapshot-e2e.nix",
    register_flake_check = false,
  },
}
