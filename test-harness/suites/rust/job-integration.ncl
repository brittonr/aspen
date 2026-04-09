{
  id = "job-integration",
  display_name = "Job integration tests",
  description = "Real-cluster job queue and worker execution coverage through root integration tests.",
  layer = "rust-integration",
  owner = "jobs",
  runtime_class = "real-network",
  prerequisites = ["network-access"],
  tags = ["jobs", "rpc", "real-cluster"],
  target = {
    kind = "cargo-nextest",
    package = "aspen",
    test = "job_integration_test",
    profile = "network",
    features = ["jobs"],
    run_ignored = "all",
  },
}
