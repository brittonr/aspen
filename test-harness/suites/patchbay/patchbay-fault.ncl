{
  id = "patchbay-fault",
  display_name = "Patchbay fault injection tests",
  description = "Namespace-backed patchbay coverage for partitions, packet loss, and recovery.",
  layer = "patchbay",
  owner = "networking",
  runtime_class = "linux-namespaces",
  prerequisites = ["linux-userns", "nftables", "traffic-control"],
  tags = ["patchbay", "fault-injection", "recovery"],
  target = {
    kind = "cargo-nextest",
    package = "aspen-testing-patchbay",
    test = "patchbay_fault_tests",
    profile = "patchbay",
  },
}
