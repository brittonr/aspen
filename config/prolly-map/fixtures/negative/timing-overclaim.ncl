let Base = import "../../benchmark.ncl" in
let Contracts = import "../../benchmark-contracts.ncl" in
(Base & { raw_timings_are_correctness | force = true }) | Contracts.Benchmark
