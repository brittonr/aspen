Evidence-ID: alloc-safe-hooks-ticket.v1-equivalence
Task-ID: V1
Artifact-Type: comparison
Covers: architecture.modularity.alloc-safe-hook-tickets-default-to-transport-neutral-bootstrap-metadata.default-and-explicit-alloc-safe-surfaces-remain-equivalent

# Default vs no-default equivalence

## Normal graph comparison

- Status: identical
- Compared: `cargo tree -p aspen-hooks-ticket -e normal` vs `cargo tree -p aspen-hooks-ticket --no-default-features -e normal`

## Feature graph comparison

- Status: identical
- Compared: `cargo tree -p aspen-hooks-ticket -e features` vs `cargo tree -p aspen-hooks-ticket --no-default-features -e features`
