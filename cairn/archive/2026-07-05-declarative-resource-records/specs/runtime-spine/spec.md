# Runtime Spine Delta: declarative resource records

### Requirement: Canonical resource records bind desired and observed state
r[molten.resource_model.canonical_resource_records] Molten MUST represent declarative runtime resources as canonical Preserves records that bind resource type, resource ref, scope ref, scoped name, generation, desired-state ref, optional observed-state ref, metadata, and evidence refs. Molten MUST NOT claim Kubernetes API, YAML, CRD, controller-runtime, or storage compatibility for these records.

#### Scenario: Valid resource identity is stable
- GIVEN a resource record with a reviewed resource type, scope ref, scoped name, generation, desired-state ref, metadata, and evidence refs
- WHEN Molten canonicalizes the record
- THEN the resource ref is computed from canonical Preserves identity bytes
- AND the same logical identity produces the same resource ref during replay.

#### Scenario: Malformed resource metadata denies
- GIVEN a resource record with a malformed ref, duplicate scoped identity, invalid label, unsupported type version, or non-canonical identity bytes
- WHEN resource admission validates the record
- THEN Molten denies before the record becomes live runtime state
- AND diagnostics identify the invalid identity or metadata binding.

### Requirement: Status conditions bind observed generation
r[molten.resource_model.status_conditions_observed_generation] Molten MUST report declarative resource status through condition records that bind observed generation, condition type, status value, reason, severity, message, transition evidence refs, and optional observed-state refs. Status updates MUST NOT mutate desired state.

#### Scenario: Current observed generation updates status
- GIVEN a live resource at generation `current_generation` and a controller observation for that same generation
- WHEN the status update carries condition evidence for that generation
- THEN Molten records the condition update and binds it to the observed generation.

#### Scenario: Stale status update denies
- GIVEN a resource whose desired generation has advanced beyond a controller observation
- WHEN the stale controller attempts to write status for the old generation as current
- THEN Molten denies the current-status claim
- AND the desired-state ref remains unchanged.

### Requirement: Owner refs and finalizers gate deletion and GC
r[molten.resource_model.owner_refs_finalizers_gc] Molten MUST treat owner refs, finalizers, pins, retention policy refs, and cleanup receipt refs as explicit deletion and GC inputs. Deletion or GC MUST deny while a required finalizer, live owner, pin, retention hold, or authority check remains unresolved.

#### Scenario: Finalized resource can be deleted
- GIVEN a resource marked for deletion with all finalizer cleanup receipts, no live owner blockers, no retention holds, and valid deletion authority
- WHEN deletion eligibility is evaluated
- THEN Molten emits a deletion-ready decision binding the cleanup receipts and authority evidence.

#### Scenario: Missing cleanup receipt blocks deletion
- GIVEN a resource marked for deletion with an outstanding finalizer and no matching cleanup receipt
- WHEN deletion eligibility is evaluated
- THEN Molten denies deletion or GC
- AND diagnostics identify the unresolved finalizer.
