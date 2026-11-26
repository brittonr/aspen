## Context

Seven source-vs-spec discrepancies were found during companion-doc review and left as article notes. Each is small and local, but each lets a reader or caller rely on a guarantee the code does not provide. Fixing them in one change keeps the companion articles' discrepancy section honest.

## Decisions

### Decision: Overlap is verification-only

**Choice:** Remove the `SignAndVerify` overlap mode. `KeyCurrentness::Overlap` no longer satisfies `permits_signing`, so `plan_sign` denies overlap signing with a typed diagnostic.

**Rationale:** The governing identity document already defines overlap as verification-only, and the file adapter already refuses overlap. Aligning the core removes a signing path that nothing is allowed to use.

### Decision: Validate the deadline domain where the deadline is built

**Choice:** `plan_retry` calls the same deadline-domain validator as `evaluate_deadline` before returning a `Deadline`.

**Rationale:** Rejecting a wall-clock base at construction keeps the evaluator check as defence in depth, not the only barrier.

### Decision: Entropy evidence binds the capability reference

**Choice:** Add `capability_ref` to `EntropyEvidenceMetadata` and its canonical event under a new versioned evidence label. Replay rejects an event whose capability reference does not match the admitted stream.

**Rationale:** The governing prose is the stronger contract. Binding the reference in evidence lets replay check it without trusting shell-side state.

### Decision: Missing descriptor bridge is `Unsupported`

**Choice:** Map a missing or unreadable `/proc/self/fd` entry to the `Unsupported` error class on every Unix host. Other I/O errors keep their class.

**Rationale:** Callers branch on the failure class. A missing bridge is a capability gap, not a transient filesystem fault.

### Decision: Version the snapshot receipt check label

**Choice:** Emit `readable-assertions-subset-admitted` in a new receipt version, and keep parsing the old label only as an alias of the same check.

**Rationale:** A receipt must not claim containment in a set it never checked. Versioning preserves canonical identity of existing receipts.

### Decision: Public key status carries the opaque handle reference

**Choice:** Add the opaque handle reference to the canonical public status record under a new versioned label. The reference identifies the handle and reveals no key material.

**Rationale:** The governing status prose already promises this field, and consumers need it to tell rotated generations apart.

### Decision: The default capability context grants nothing

**Choice:** `CapabilityContext::default()` returns an empty grant set. Callers that need allow-all must call `allow_all()` explicitly, and every current `Default` caller is reviewed and migrated.

**Rationale:** Deny-by-default is the architecture's rule. An allow-all `Default` turns any forgotten initialization into full authority.

## Risks / Trade-offs

- The entropy evidence, snapshot receipt, and key status changes alter canonical bytes for newly produced records. Each uses a new versioned label so older records keep their identity.
- Changing `CapabilityContext::default()` can deny callers that relied on implicit allow-all; each `Default` caller must be migrated in the same commit.
