let Base = import "../../benchmark.ncl" in
let Contracts = import "../../benchmark-contracts.ncl" in
(Base & { creates_repository | force = true }) | Contracts.Benchmark
