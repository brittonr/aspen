mod decision;
mod profile;
mod result;

pub use decision::*;
pub use profile::*;
pub use result::*;

pub const WORLD_BENCHMARK_PROFILE_SCHEMA: &str = "molten.world-benchmark-profile.v1";
pub const WORLD_BENCHMARK_PLAN_SCHEMA: &str = "molten.world-benchmark-plan.v1";
pub const WORLD_BENCHMARK_RECEIPT_SCHEMA: &str = "molten.world-benchmark-receipt.v1";
pub const WORLD_BENCHMARK_COMPARISON_SCHEMA: &str = "molten.world-benchmark-comparison.v1";
pub const WORLD_BENCHMARK_EXTRACTION_SCHEMA: &str = "molten.world-benchmark-extraction-decision.v1";
pub const WORLD_BENCHMARK_PLAN_DOMAIN: &str = "molten.world-benchmark-plan.v1";
pub const WORLD_BENCHMARK_RECEIPT_DOMAIN: &str = "molten.world-benchmark-receipt.v1";
pub const CHAOSCONTROL_SNAPSHOT_REVISION: &str = "b8c440ea3b19df796542e58e8ee36200e1c3db85";
pub const CHAOSCONTROL_SNAPSHOT_PROFILE: &str = "exact-x86-kvm-v1";
pub const MAX_WORLD_BENCHMARK_TEXT_BYTES: usize = 256;
pub const MAX_WORLD_BENCHMARK_OPERATIONS: usize = 16;
pub const MAX_WORLD_BENCHMARK_REPETITIONS: u32 = 16;
pub const MAX_WORLD_BENCHMARK_RESULTS: usize = 256;
pub const MAX_WORLD_BENCHMARK_ADAPTERS: usize = 16;
pub const MAX_WORLD_BENCHMARK_THRESHOLDS: usize = 32;
pub const WORLD_BENCHMARK_METRIC_COUNT: usize = 12;

pub const WORLD_BENCHMARK_NON_CLAIMS: &[&str] = &[
    "finite benchmark runs do not prove asymptotic complexity",
    "benchmark observations do not prove universal performance or storage correctness",
    "opaque and logical cohorts are not semantically or performance equivalent",
    "retention metrics and deletion plans do not grant deletion authority",
    "benchmark receipts do not grant activation, dependency, extraction, or release authority",
];

pub fn world_benchmark_non_claims() -> Vec<String> {
    WORLD_BENCHMARK_NON_CLAIMS.iter().map(ToString::to_string).collect()
}
