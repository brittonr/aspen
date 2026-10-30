{
  schema = "molten-nix-source-binding-v1",
  gate_profile = "strict-ci",
  required_runs = ["workspace", "molten-library", "object-corpus", "strict-gate", "remediation-plan"],
  closure_roles = ["candidate-build", "candidate-compiler", "candidate-runtime", "gate-build", "gate-compiler", "gate-runtime"],
  required_binaries = ["molten", "molten-native-extension-fixture", "molten-collaboration-scope"],
  authentication = "independently-authorized-nix-output-signature",
  nonclaims = ["compiler correctness", "production readiness", "runtime authority"],
}
