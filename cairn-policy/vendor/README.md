# Cairn runtime policy contract snapshot

Source: https://github.com/OnixResearch/cairn
Revision: `15f00875562025e7ea7e0d1f4af24d1a2e2ac06f`

The four Nickel files are exact Git-tree copies. Both upstream license files remain present. OnixResearch authorizes this source reuse. The public licenses do not change.

Cairn does not publish a standalone complete policy-contract package. This consumer snapshot avoids a runtime network fetch or an ambient sibling dependency. No Rust source is copied. Molten's embedded Cairn dependency stays at its existing revision.

`../default.ncl` selects each runtime field explicitly. It does not merge an upstream default over Molten policy. Regression tests preserve the committed gate settings, exemptions, trust hashes, replay cases, and receipt contracts. The committed projection already allowed three workflow profiles; that list remains unchanged. This change uses `spec-driven`.

The migration adds exact-marker traceability fields and preserves the committed `.cairn/` lifecycle paths. The old authored file lagged the committed projection. Five missing receipt-schema rows return without deleting their existing contract references. Retained `cairn/archive/` history is not moved or used as a fallback. Regression tests compare the prior policy snapshot from Molten commit `9ae893f31a0f042e533a65e4b21ede0482ae6c4a` with the generated candidate.

The compatibility target is the separately selected operator Cairn CLI. Passing its gates does not establish full Molten workspace, Octet, release, or normal-service VM acceptance.
