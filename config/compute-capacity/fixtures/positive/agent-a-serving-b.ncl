# Synthetic authoring inputs only: no actual devices, deployed model or admitted grants.
let Contracts = import "../../contracts.ncl" in
let Profile = import "../../profile.ncl" in
# Opaque synthetic reference shapes; no live provenance or qualification is asserted.
let synthetic_provider_ref = fun host => if host == "host-a" then "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" else "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb" in
let synthetic_source_ref = fun host => if host == "host-a" then "blake3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc" else "blake3:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd" in
let synthetic_qualification_ref = fun host => if host == "host-a" then "blake3:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee" else "blake3:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff" in
let synthetic_grant_ref = fun operation => if operation == "serving-b" then "blake3:1111111111111111111111111111111111111111111111111111111111111111" else if operation == "agent-a-1" then "blake3:2222222222222222222222222222222222222222222222222222222222222222" else "blake3:3333333333333333333333333333333333333333333333333333333333333333" in
let pool = fun pool_id pool_kind capacity => {
  id = pool_id,
  kind = pool_kind,
  authority_id = "synthetic-owner-" ++ pool_id,
  total = capacity,
  allocatable = capacity,
  observed_used = null,
  observed_free = capacity,
  is_observation_debited = false,
} in
let view = fun view_name device grouping physical_pools => {
  id = view_name,
  device_id = device,
  partition_id = null,
  granularity = grouping,
  backing_pools = physical_pools,
  is_grouping_enforced = true,
} in
let offer = fun host runtime_name physical_pools device_views => {
  schema = "molten.compute.offer.v1",
  host_id = host,
  host_incarnation = "synthetic-host-generation-1",
  device_incarnation = "synthetic-device-generation-1",
  provider_profile_ref = synthetic_provider_ref host,
  source_profile_ref = synthetic_source_ref host,
  revision = 1,
  topology_generation = 1,
  observed_at = 10,
  observation_charge_cursor = 0,
  expires_at = 100,
  health = "healthy",
  label_authority = "authoritative",
  compatibility = {
    backend = "synthetic-backend",
    runtime = runtime_name,
    architecture = "synthetic-architecture",
    topology = "single",
    qualification_refs = [synthetic_qualification_ref host],
  },
  pools = physical_pools,
  views = device_views,
} in
let make_request = fun host operation consumer runtime_name requested_view grouping requested_claims serving_id allocation_kind => {
  host_id = host,
  host_incarnation = "synthetic-host-generation-1",
  device_incarnation = "synthetic-device-generation-1",
  offer_revision = 1,
  topology_generation = 1,
  now = 20,
  view_id = requested_view,
  granularity = grouping,
  compatibility = {
    backend = "synthetic-backend",
    runtime = runtime_name,
    architecture = "synthetic-architecture",
    topology = "single",
  },
  minimum_label_authority = "authoritative",
  consumer_id = consumer,
  operation_id = operation,
  attempt_id = "attempt-" ++ operation,
  owner_generation = 1,
  grant_ref = synthetic_grant_ref operation,
  grant = {
    grant_ref = synthetic_grant_ref operation,
    consumer_id = consumer,
    is_valid = true,
    is_revoked = false,
    expires_at = 80,
  },
  claims = requested_claims,
  serving_allocation_id = serving_id,
  kind = allocation_kind,
} in
{
  schema = "molten.compute.synthetic-consumer.v1",
  profile_id = Profile.profile_id,
  offers = [
    offer "host-a" "cuda" [
      pool "host-a-memory" "unified-memory-bytes" 100,
      pool "host-a-device-memory" "dedicated-memory-bytes" 100,
      pool "host-a-chip" "chips" 1,
    ] [
      view "host-a-agent-execution" "host-a-cpu" "alias" ["host-a-memory"],
      view "host-a-device" "host-a-device" "mmio-bundle" ["host-a-device-memory", "host-a-chip"],
    ],
    offer "host-b" "unified" [
      pool "host-b-shared-memory" "unified-memory-bytes" 100,
      pool "host-b-chip" "chips" 1,
    ] [
      view "host-b-serving" "host-b-device" "mmio-bundle" ["host-b-shared-memory", "host-b-chip"],
    ],
  ],
  steps = [
    {
      expected_allocation_id = "serving-b",
      request = make_request "host-b" "serving-b" "serving-operator" "unified" "host-b-serving" "mmio-bundle" [
        { pool_id = "host-b-shared-memory", quantity = 80 },
        { pool_id = "host-b-chip", quantity = 1 },
      ] null "serving-deployment",
    },
    {
      expected_allocation_id = "agent-a-1",
      request = make_request "host-a" "agent-a-1" "agent-1" "cuda" "host-a-agent-execution" "alias" [
        { pool_id = "host-a-memory", quantity = 40 },
      ] "serving-b" "execution",
    },
    {
      expected_allocation_id = "agent-a-2",
      request = make_request "host-a" "agent-a-2" "agent-2" "cuda" "host-a-agent-execution" "alias" [
        { pool_id = "host-a-memory", quantity = 40 },
      ] "serving-b" "execution",
    },
  ],
  synthetic_only = true,
} | Contracts.Consumer
