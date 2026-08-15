# Project Delta: Public API Surface Tightening

### Requirement: Public modules are classified
r[molten.modularity.public_api.classified_surface] Public root-crate modules and re-exports SHOULD be classified as stable API, compatibility alias, internal implementation, or generated/test support before modularity refactors remove or hide them.

#### Scenario: Public export has intent
- GIVEN a public module, compatibility alias, or re-export in the root crate
- WHEN the API inventory is reviewed
- THEN the export is classified with its intended stability and migration status

#### Scenario: Unclassified public export blocks removal
- GIVEN a public export lacks a stability classification
- WHEN a refactor proposes to remove, rename, or hide it
- THEN the change records a classification first or defers the removal to a compatibility-owned change

### Requirement: Stable API surface is intentional
r[molten.modularity.public_api.intentional_exports] The repository SHOULD expose a small intentional API or prelude for stable consumers and SHOULD avoid making implementation modules public solely for internal convenience.

#### Scenario: Preferred API is discoverable
- GIVEN a consumer needs stable Molten core types or constructors
- WHEN they inspect public documentation or the root API module
- THEN the preferred stable import path is identifiable without relying on compatibility aliases

#### Scenario: Compatibility alias is not preferred
- GIVEN a compatibility alias remains for migration
- WHEN new internal code is added
- THEN it uses the preferred stable or crate-internal path instead of expanding use of the compatibility alias

### Requirement: Implementation visibility is minimized
r[molten.modularity.public_api.visibility] Implementation details SHOULD be private or `pub(crate)` unless they are required for the reviewed public API, canonical artifact parsing, fixture support, or compatibility migration.

#### Scenario: Internal helper is hidden
- GIVEN an implementation helper has no external compatibility requirement
- WHEN modularity cleanup touches its owning module
- THEN the helper becomes private or `pub(crate)` while existing tests and consumers continue to compile

#### Scenario: Required public helper records reason
- GIVEN an implementation-looking helper must remain public
- WHEN reviewers inspect the API inventory
- THEN the reason is recorded as stable API, fixture support, generated boundary, or compatibility migration

### Requirement: API surface changes are validated
r[molten.modularity.public_api.validation] Public API tightening SHOULD include compile checks, tests, or policy checks proving intended public paths still work and accidental implementation exports do not expand.

#### Scenario: Intended public API compiles
- GIVEN the preferred public API surface after cleanup
- WHEN compile or UI checks run
- THEN representative imports and calls for the intended surface succeed

#### Scenario: Accidental public surface is detected
- GIVEN a new implementation module is exported publicly without classification
- WHEN API surface validation runs
- THEN validation fails or records the unclassified export before release evidence is promoted
