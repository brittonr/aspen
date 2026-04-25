# Bridge, Gateway, Web, and TUI Compatibility Evidence
Generated: 2026-04-25T02:22:06Z

## cargo check -p aspen-cluster-bridges
    Checking aspen-cluster-bridges v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster-bridges)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.13s

## cargo check -p aspen-snix-bridge
warning: `openraft` (lib) generated 4 warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.47s

## cargo check -p aspen-nix-cache-gateway
warning: `openraft` (lib) generated 4 warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.41s

## cargo check -p aspen-h3-proxy
warning: `openraft` (lib) generated 4 warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.35s

## cargo check -p aspen-forge-web
warning: `aspen-client-api` (lib) generated 28 warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.37s

## cargo check -p aspen-tui
warning: `aspen-client-api` (lib) generated 28 warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.35s
