let Contracts = import "contracts.ncl" in
let Base = import "valid.ncl" in
Base & {
  measurements = [{
    artifact_family = "node-control-envelope",
    canonical_bytes = true,
    blake3_ref = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    consumer = "runtime",
    raw_preserves_core_coupling = false,
  }],
} | Contracts.Profile
