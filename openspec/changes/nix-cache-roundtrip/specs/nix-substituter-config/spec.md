## MODIFIED Requirements

### Requirement: CI executor auto-configures substituter

The CI Nix executor SHALL automatically configure `nix build` to use the aspen cache as a substituter when the cache gateway is available.

#### Scenario: Cache available via gateway URL

- **WHEN** a CI build starts and `gateway_url` is Some and `cache_public_key` is Some
- **THEN** the executor SHALL add `--extra-substituters {gateway_url}` and `--trusted-public-keys {key}` to the `nix build` invocation

#### Scenario: Cache available via cache proxy

- **WHEN** a CI build starts and `use_cluster_cache` is true and `can_use_cache_proxy()` returns true and `gateway_url` is None
- **THEN** the executor SHALL start a local cache proxy, add `--substituters http://127.0.0.1:{port}` and `--trusted-public-keys {key}` to the `nix build` invocation

#### Scenario: Cache unavailable

- **WHEN** a CI build starts and `can_use_cache_proxy()` returns false and `gateway_url` is None
- **THEN** the executor SHALL proceed without the aspen substituter (graceful degradation)

#### Scenario: Cache proxy failure during build

- **WHEN** the cache proxy crashes or becomes unresponsive during a build
- **THEN** `nix build` SHALL fall through to other configured substituters (e.g., cache.nixos.org) without failing the build
