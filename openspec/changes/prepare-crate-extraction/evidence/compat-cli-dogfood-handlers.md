# CLI, Dogfood, and Handler Compatibility Evidence
Generated: 2026-04-25T02:20:37Z

## cargo check -p aspen-cli
warning: `aspen-cli` (bin "aspen-cli") generated 6 warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 9.08s

## cargo check -p aspen-dogfood
    Checking aspen-dogfood v0.1.0 (/home/brittonr/git/aspen/crates/aspen-dogfood)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.44s

## cargo check -p aspen-rpc-handlers
warning: `aspen-client-api` (lib) generated 28 warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.32s

## cargo check -p aspen-core-essentials-handler
    Checking aspen-core-essentials-handler v0.1.0 (/home/brittonr/git/aspen/crates/aspen-core-essentials-handler)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.73s

## cargo check -p aspen-cluster-handler
    Checking aspen-cluster-handler v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster-handler)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.18s

## cargo check -p aspen-blob-handler
    Checking aspen-blob-handler v0.1.0 (/home/brittonr/git/aspen/crates/aspen-blob-handler)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 7.10s

## cargo check -p aspen-forge-handler
    Checking aspen-forge-handler v0.1.0 (/home/brittonr/git/aspen/crates/aspen-forge-handler)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 11.58s

## cargo check -p aspen-docs-handler
    Checking aspen-docs-handler v0.1.0 (/home/brittonr/git/aspen/crates/aspen-docs-handler)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.51s

## cargo check -p aspen-secrets-handler
    Checking aspen-secrets-handler v0.1.0 (/home/brittonr/git/aspen/crates/aspen-secrets-handler)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.63s

## cargo check -p aspen-ci-handler
    Checking aspen-ci-handler v0.1.0 (/home/brittonr/git/aspen/crates/aspen-ci-handler)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 13.00s

## cargo check -p aspen-job-handler
    Checking aspen-job-handler v0.1.0 (/home/brittonr/git/aspen/crates/aspen-job-handler)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.55s

## cargo check -p aspen-nix-handler
    Checking aspen-nix-handler v0.1.0 (/home/brittonr/git/aspen/crates/aspen-nix-handler)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.34s

