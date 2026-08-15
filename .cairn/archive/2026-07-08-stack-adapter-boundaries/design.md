## Context

Molten is explicitly policy-gated and evidence-bearing, but the dependency graph currently exposes many stack internals directly to the runtime crate. That is useful during bootstrapping but weakens review boundaries: a runtime decision should cite an admitted evidence ref, not imply full trust in an upstream repository checkout.

## Design

### Stack evidence envelope

Define a canonical envelope for stack inputs. Each member names:

- role, such as `basalt-policy-decision`, `ucan-authorization-semantics`, `trellis-primitive-manifest`, `octet-provenance-export`, `valence-evidence-ir`, `cairn-lifecycle-receipt`, or `mantle-release-evidence`;
- schema string;
- BLAKE3 digest or content ref;
- producer repo/revision or manifest digest;
- verification role;
- non-claim boundary.

### Adapter ports

Runtime pure cores consume only parsed DTOs and refs. Shell adapters perform upstream-specific parsing, command invocation, filesystem reads, and receipt verification. Adapter code must convert upstream evidence into Molten envelope facts before core admission.

### Dependency-boundary checks

Add a static or metadata check that reports stack-owned crates used outside approved adapter modules. The check is diagnostic at first and can later become a release gate once the migration is complete.

## Alternatives

### Keep direct dependencies everywhere

Rejected. It keeps prototypes simple but makes authority and evidence boundaries hard to audit.

### Remove all upstream dependencies immediately

Rejected. Some crates are still useful implementation dependencies; the key is to confine them behind reviewed adapters and evidence envelopes.

## Risks

- **Migration churn**: start with envelope contracts and diagnostic dependency checks before moving every call site.
- **Overbroad evidence claims**: require non-claims on every envelope member.
- **Adapter complexity**: keep adapter ports narrow and typed around the specific runtime decisions they support.
