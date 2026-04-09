{
  id = "blob-replication",
  display_name = "Blob replication integration tests",
  description = "Real-network blob replication coverage for root integration tests.",
  layer = "rust-integration",
  owner = "blob",
  runtime_class = "real-network",
  prerequisites = ["network-access"],
  tags = ["blob", "replication", "real-cluster"],
  target = {
    kind = "cargo-nextest",
    package = "aspen",
    test = "blob_replication_integration_test",
    profile = "network",
    features = ["blob"],
    run_ignored = "all",
  },
}
