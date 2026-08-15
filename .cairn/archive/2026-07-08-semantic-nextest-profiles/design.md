## Context

`.config/nextest.toml` contains default, CI, deterministic, and exploratory profiles. `docs/distributed-testing.md` describes a risk matrix, but the mapping from test subsets to evidence scope is not fully executable.

## Design

Add semantic profiles or Nix wrapper checks that map test subsets to explicit evidence scopes:

- fast core: pure units, parsers, receipts, and deterministic helpers;
- harness: report, replay, gate, repro, redaction, and failure artifact behavior;
- CLI: command integration and receipt writing;
- distributed simulation: deterministic multi-peer and fault fixtures;
- VM/platform: NixOS VM and platform integration evidence;
- dogfood/soak: operator-readiness evidence only.

The pure profile-classification core should derive profile metadata from typed config: profile id, command surface, expected artifacts, retry policy, release scope, cost class, and caveats. Shells own nextest invocation, Nix check execution, JUnit paths, and documentation rendering.

## Validation

Validation should cover profile metadata parsing, profile-to-command mapping, retry policy enforcement, unavailable platform handling, and exploratory exclusion from deterministic evidence. Nix nextest config checks should continue to verify the generated profile names and JUnit paths.
