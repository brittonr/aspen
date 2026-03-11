{
  name = "aspen-full-build",
  stages = [
    {
      name = "build",
      jobs = [
        {
          name = "build-aspen-node",
          type = 'vm,
          command = "nix",
          args = ["build", "-L", ".#default"],
          flake_attr = "default",
          timeout_secs = 1800,
        },
      ],
    },
  ],
}
