## Why

Modularity rules are easy to state and easy to regress. Without automated boundary gates, core modules can import adapters, runtime modules can import CLI shells, and compatibility aliases can spread back into new code.

## What Changes

- Add repository-owned dependency-boundary checks for source-layer rules.
- Encode rules such as core not importing adapters, runtime not importing CLI, codec not importing high-level domains, and CLI not owning pure domain decisions.
- Report readable diagnostics tied to the offending file and rule.
- Add positive and negative fixtures or smoke checks for the boundary validator.

## Impact

This change turns modularity guidance into enforceable evidence. It does not by itself move code, but it prevents future refactors from reintroducing high-risk coupling.
