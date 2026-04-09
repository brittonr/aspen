{
  id = "patchbay-nat",
  display_name = "Patchbay NAT topology tests",
  description = "Namespace-backed patchbay coverage for NAT and leader election scenarios.",
  layer = "patchbay",
  owner = "networking",
  runtime_class = "linux-namespaces",
  prerequisites = ["linux-userns", "nftables", "traffic-control"],
  tags = ["patchbay", "nat", "leader-election"],
  target = {
    kind = "cargo-nextest",
    package = "aspen-testing-patchbay",
    test = "patchbay_nat_tests",
    profile = "patchbay",
  },
}
