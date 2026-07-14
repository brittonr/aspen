## Context

Molten is both a network runtime and an AGPL-component consumer. A single AGPL project license aligns the source declaration with the distributed binary boundary and removes the clean-room distinction that was only needed to preserve a permissive Molten fork.

## Decisions

### Use AGPL-3.0-or-later for Molten-owned source

Cargo workspace metadata, generated package metadata, the checked-in license text, and current user-facing documentation use `AGPL-3.0-or-later`.

### Keep third-party licenses intact

`vendor/`, dependency sources, generated dependency inventories, and referenced upstream artifacts retain their original license expressions, copyright notices, and notice obligations. The project license does not overwrite those terms.

### Do not overstate retroactivity

The new release boundary does not revoke licenses already granted for earlier copies and does not convert upstream code into Molten-owned AGPL code.

### Keep release-profile automation separate

This change establishes the present source/package declaration. The active `pin-canonical-evidence-dependencies` change still owns typed distribution-profile evidence, immutable dependency pins, and canonical Valence migration.
