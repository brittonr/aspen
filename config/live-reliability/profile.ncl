let Contracts = import "contracts.ncl" in
// Product profile: embed this value inside an exact Onix deployment cohort.
// Call with measured canonical product identities; there are no default artifact identities.
fun identities =>
{
  schema = "molten.live-register-profile.v1",
  group = "live-register-group",
  service = "service-registry",
  keys = ["live-register"],
  initial_value = null,
  values = [1, 2, 3],
  generator_ref = identities.generator_ref,
  adapter_ref = identities.adapter_ref,
  model_ref = identities.model_ref,
  outcome_ref = identities.outcome_ref,
  seed = 29,
  read_weight = 1,
  write_weight = 1,
  clients = ["client-a", "client-b"],
  endpoints = ["node-a:7180", "node-b:7180", "node-c:7180"],
  concurrency = 2,
  operations = 32,
  control_operations = 8,
  consistency_model = "register",
  recovery = {
    timeout_ms = 30000,
    stable_membership_observations = 3,
    require_all_endpoint_reads = true,
  },
  evidence = {
    max_attempts_per_operation = 3,
    max_events = 256,
    max_uncertain = 16,
    max_bytes = 8388608,
    checker_states = 50000,
    checker_branches = 100000,
    checker_memo_bytes = 8388608,
    checker_reductions = 256,
  },
} | Contracts.Profile
