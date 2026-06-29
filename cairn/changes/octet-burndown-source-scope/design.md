## Context

No-disabled probes still report some `module_file_count` and `underscore_in_module_filename` findings for paths that look like `<WORKSPACE>/src/...` but appear to originate from external dependencies, generated source maps, or remapped registry/rustlib code. Molten-owned source should stay fail-closed; external-path noise needs explicit classification instead of ad hoc suppression.

## Design

### Classification core

Represent each no-disabled finding classification as deterministic data:

- finding id and lint name;
- reported path and crate;
- classification: Molten-owned source, integration-test source, generated/remapped dependency source, registry/rustlib source, or unknown;
- evidence explaining the classification;
- decision: actionable, ignored as external, or blocked pending tooling.

The classification logic should be pure over parsed summary/index data and repository path inventories. Any filesystem reads, Octet invocation, and report writing stay in a thin shell.

### Tooling boundary

Octet configuration or tooling changes may narrow source scope only when they are explicit and evidence-backed. Unknown findings must remain actionable or blocked; they must not silently disappear.

### Validation

Validation should include a no-disabled probe, classification output, and a check that known Molten-owned source findings still report. Any source-scope change must be documented before disabled lint families are removed or narrowed.

### Non-goals

- Do not weaken Octet gates for Molten-owned source.
- Do not treat source-map ambiguity as source-remediated-zero evidence.
- Do not remove `module_file_count` caveats until classification is complete and validated.
