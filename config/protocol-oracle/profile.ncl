let Contracts = import "contracts.ncl" in
let service_entry = fun service_name position =>
  {
    service = service_name,
    logical_position = position,
    safety = ["local", "durability"],
    physical_layout = "semantic-only",
  }
in
let participant_entry = fun service_name =>
  {
    participant = "node-%{service_name}",
    service = service_name,
    is_required = true,
  }
in
let counter_entry = fun counter_name => { name = counter_name, resets_on_generation = false } in
{
  schema = "molten.protocol-oracle-profile.v1",
  profile_id = "reference-protocol-oracle",
  projection_schema = "molten.protocol-projection.v1",
  oracle = { oracle_id = "molten.reference-protocol-oracle", version = 1 },
  oracle_ref = "blake3:14bac5bfb5dfd690086f25ff24e9b1865cd22cec314ccf08835a807151a5dbb4",
  services = [
    service_entry "transactional-key-value" "version",
    service_entry "replicated-log" "next-offset",
    service_entry "distributed-scheduler" "transition-count",
  ],
  participants = [
    participant_entry "transactional-key-value",
    participant_entry "replicated-log",
    participant_entry "distributed-scheduler",
  ],
  completeness = "lossless",
  liveness = {
    requires_readiness = true,
    requires_membership = true,
    disruptive_faults_block = true,
    requires_durability = true,
    virtual_progress_horizon = 64,
  },
  novelty_fields = ["service", "transition-class", "logical-position", "state"],
  counters = [
    counter_entry "transitions",
    counter_entry "port-events",
    counter_entry "storage-operations",
    counter_entry "copied-bytes",
  ],
  bounds = {
    max_records = 4096,
    max_state_bytes = 65536,
    max_total_bytes = 16777216,
    max_participants = 16,
    max_counters = 16,
  },
  non_claims = Contracts.required_non_claims,
} | Contracts.Profile
