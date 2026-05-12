## Phase 1: Manifest

- [x] [serial] Inventory `aspen-nickel` and `aspen-plugin-api` features, dependencies, and current consumers.
- [x] [depends:inventory] Write `docs/crate-extraction/config-plugin.md` with owner, feature contract, examples, consumers, exceptions, and rails.

## Phase 2: Policy and examples

- [x] [depends:manifest] Add policy/inventory rows while keeping readiness `workspace-internal`.
- [x] [depends:manifest] Add or document standalone example checks for config parsing and plugin API protocol/types.
- [x] [depends:examples] Add checker expectations for missing manifest/owner/evidence failures if the family is later promoted.

## Phase 3: Closeout

- [x] [depends:policy] Run the manifest/readiness checker, strict OpenSpec validation, and `git diff --check`.
