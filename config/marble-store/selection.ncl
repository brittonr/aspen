let Contracts = import "contracts.ncl" in
{
  schema = "molten.prolly-store-selection.v1",
  backend = "redb",
  marble = {
    catalog_ids = { start = 0, end = 1 },
    block_ids = { start = 1024, end = 4398046511104 },
    maintenance_every_committed_batches = 64,
    executor_queue_depth = 64,
    read_cache_entries = 4096,
    max_object_bytes = 1048576,
  },
  non_claims = Contracts.required_non_claims,
} | Contracts.ReviewedSelection
