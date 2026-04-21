# uhcl patches

## no-rand alloc-only support

Aspen's `aspen-core --no-default-features` boundary requires the alloc-only graph to avoid implicit randomness dependencies.

This vendor patch makes `uhlc`'s `rand` dependency optional, keeps upstream default behavior by enabling `rand` in `default`, and lets `std` build without forcing `rand`. When `rand` is disabled, `ID::rand()` falls back to a fixed non-zero identifier so `HLCBuilder::default()` still compiles for callers that immediately override the ID, which is how Aspen uses `uhlc` through `aspen-hlc`.
