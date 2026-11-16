# Authored expected intent record, not a persisted observation.
let Contracts = import "../../contracts.ncl" in
let Example = import "agent-a-serving-b.ncl" in
let Serving = (std.array.first Example.steps).request in
{
  schema = "molten.compute.reservation.v1",
  id = Serving.operation_id,
  host_id = Serving.host_id,
  host_incarnation = Serving.host_incarnation,
  device_incarnation = Serving.device_incarnation,
  offer_revision = Serving.offer_revision,
  topology_generation = Serving.topology_generation,
  charge_cursor = 1,
  valid_until = 80,
  authority_ids = ["synthetic-owner-host-b-shared-memory", "synthetic-owner-host-b-chip"],
  view_id = Serving.view_id,
  granularity = Serving.granularity,
  consumer_id = Serving.consumer_id,
  operation_id = Serving.operation_id,
  attempt_id = Serving.attempt_id,
  owner_generation = Serving.owner_generation,
  grant_ref = Serving.grant_ref,
  claims = Serving.claims,
  serving_allocation_id = Serving.serving_allocation_id,
  kind = Serving.kind,
  status = "intent",
  enforcement = "logical-fence-only",
  activation_backend_ref = null,
} | Contracts.Allocation
