let TT = import "../positive/tt-overlap.ncl" in
let Example = import "../positive/agent-a-serving-b.ncl" in
let Contracts = import "../../contracts.ncl" in
let Serving = std.array.first Example.steps in
{
  schema = TT.schema,
  profile_id = TT.profile_id,
  offers = TT.offers,
  steps = [{
    expected_allocation_id = Serving.expected_allocation_id,
    request = Serving.request & {
      view_id | force = "tt-mesh-1-2",
      granularity | force = "mesh",
      claims | force = [
        { pool_id = "tt-chip-1", quantity = 1 },
        { pool_id = "tt-chip-2", quantity = 1 },
      ],
    },
  }],
  synthetic_only = true,
} | Contracts.Consumer
