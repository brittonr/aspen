let Contracts = import "contracts.ncl" in
let get = fun field fallback args => std.record.get_or field fallback args in
let CpuCapacityMillis = 8000 in
let MemoryCapacityBytes = 34359738368 in
let StorageCapacityBytes = 274877906944 in
let MaxRoleCpuMillis = 4000 in
let MaxRoleMemoryBytes = 17179869184 in
let MaxRoleStorageBytes = 137438953472 in
let MaxReplicas = 64 in
let MaxSearchSteps = 100000 in
let MaxViewAgeTicks = 1000 in
{
  make = fun args => {
    schema_id = Contracts.schema_id,
    schema_version = Contracts.schema_version,
    source_language = Contracts.source_language,
    source = {
      provider_kind = get "provider_kind" "static" args,
      authority_strength = get "authority_strength" "operator-declared" args,
      profile_ref = "blake3:1111111111111111111111111111111111111111111111111111111111111111",
      max_view_age_ticks = get "max_view_age_ticks" MaxViewAgeTicks args,
    },
    fencing = {
      enforcement = get "fencing_enforcement" "process-local" args,
      profile_ref = "blake3:2222222222222222222222222222222222222222222222222222222222222222",
      authority_ref = "blake3:3333333333333333333333333333333333333333333333333333333333333333",
      effect_port_ref = "blake3:4444444444444444444444444444444444444444444444444444444444444444",
    },
    max_node_capacity = {
      cpu_millis = get "node_cpu_millis" CpuCapacityMillis args,
      memory_bytes = get "node_memory_bytes" MemoryCapacityBytes args,
      storage_bytes = get "node_storage_bytes" StorageCapacityBytes args,
    },
    max_role_request = {
      cpu_millis = get "role_cpu_millis" MaxRoleCpuMillis args,
      memory_bytes = get "role_memory_bytes" MaxRoleMemoryBytes args,
      storage_bytes = get "role_storage_bytes" MaxRoleStorageBytes args,
    },
    max_replicas = get "max_replicas" MaxReplicas args,
    max_search_steps = get "max_search_steps" MaxSearchSteps args,
    non_claims = get "non_claims" Contracts.non_claims args,
  } | Contracts.Export,
}
