## ADDED Requirements

### Requirement: CI executor auto-configures substituter

The CI Nix executor SHALL automatically configure `nix build` to use the aspen cache as a substituter when the cache gateway is available.

#### Scenario: Cache available

- **WHEN** a CI build starts and `use_cluster_cache` is true and `can_use_cache_proxy()` returns true
- **THEN** the executor SHALL start a local cache proxy, add `--substituters http://127.0.0.1:{port}` and `--trusted-public-keys {key}` to the `nix build` invocation

#### Scenario: Cache unavailable

- **WHEN** a CI build starts and `can_use_cache_proxy()` returns false
- **THEN** the executor SHALL proceed without the aspen substituter (graceful degradation)

#### Scenario: Cache proxy failure during build

- **WHEN** the cache proxy crashes or becomes unresponsive during a build
- **THEN** `nix build` SHALL fall through to other configured substituters (e.g., cache.nixos.org) without failing the build

### Requirement: In-process cache proxy for CI

The CI executor SHALL embed a cache proxy that translates HTTP → aspen client RPC, replacing the H3-based proxy.

#### Scenario: Proxy starts with build

- **WHEN** a CI build needs the cache substituter
- **THEN** the executor SHALL start an in-process HTTP listener on a random localhost port and configure `nix build` to use it

#### Scenario: Proxy stops after build

- **WHEN** the CI build completes (success or failure)
- **THEN** the executor SHALL shut down the cache proxy and release the port

### Requirement: narinfo rendering from CacheEntry

The `CacheEntry` type SHALL provide a method to render itself as a valid Nix narinfo text document.

#### Scenario: Complete narinfo

- **WHEN** `to_narinfo()` is called on a CacheEntry with all fields populated
- **THEN** it SHALL return a string containing `StorePath`, `URL`, `Compression`, `NarHash`, `NarSize`, `References`, `Deriver`, and optionally `Sig` fields

#### Scenario: Minimal narinfo

- **WHEN** `to_narinfo()` is called on a CacheEntry with no references and no deriver
- **THEN** it SHALL return a valid narinfo with empty `References:` line and no `Deriver:` line

#### Scenario: Narinfo with signature

- **WHEN** `to_narinfo()` is called with a signature string
- **THEN** the narinfo SHALL include `Sig: {signature}` as the last line
