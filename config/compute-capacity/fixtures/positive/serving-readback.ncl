# Projected synthetic readback after the three authoring steps; not an executed receipt.
let Contracts = import "../../contracts.ncl" in
{
  host_id = "host-b",
  revision = 1,
  is_fresh = true,
  health = "healthy",
  qualification_refs = ["blake3:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"],
  allocations = [{
    id = "serving-b",
    consumer_id = "serving-operator",
    owner_generation = 1,
    granularity = "mmio-bundle",
    status = "intent",
    is_logically_fenced = true,
    enforcement = "logical-fence-only",
  }],
} | Contracts.Readback
