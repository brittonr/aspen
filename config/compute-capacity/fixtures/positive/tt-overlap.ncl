# Independently authored synthetic chip/tray/mesh relationships, not TT hardware evidence.
let Contracts = import "../../contracts.ncl" in
let Base = import "agent-a-serving-b.ncl" in
let B = std.array.first (std.array.filter (fun offer => offer.host_id == "host-b") Base.offers) in
let Serving = std.array.first Base.steps in
let chip = fun chip_id => {
  id = chip_id,
  kind = "chips",
  authority_id = "synthetic-physical-owner-tt-b",
  total = 1,
  allocatable = 1,
  observed_used = null,
  observed_free = 1,
  is_observation_debited = false,
} in
let grouping = fun group_id group_level backing enforced => {
  id = group_id,
  device_id = "synthetic-tt-device-b",
  partition_id = null,
  granularity = group_level,
  backing_pools = backing,
  is_grouping_enforced = enforced,
} in
{
  schema = Base.schema,
  profile_id = Base.profile_id,
  offers = [
    std.array.first (std.array.filter (fun offer => offer.host_id == "host-a") Base.offers),
    B & {
      pools | force = [chip "tt-chip-0", chip "tt-chip-1", chip "tt-chip-2"],
      views | force = [
        grouping "tt-mmio-0" "mmio-bundle" ["tt-chip-0"] true,
        grouping "tt-remote-1" "chip" ["tt-chip-1"] true,
        grouping "tt-tray-0-1" "tray" ["tt-chip-0", "tt-chip-1"] true,
        grouping "tt-mesh-0-1" "mesh" ["tt-chip-0", "tt-chip-1"] true,
        grouping "tt-mesh-1-2" "mesh" ["tt-chip-1", "tt-chip-2"] false,
      ],
    },
  ],
  steps = [
    Serving & {
      request | force = Serving.request & {
        view_id | force = "tt-mmio-0",
        claims | force = [{ pool_id = "tt-chip-0", quantity = 1 }],
      },
    },
  ],
  synthetic_only = true,
} | Contracts.Consumer
