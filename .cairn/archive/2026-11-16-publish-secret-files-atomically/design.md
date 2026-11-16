## Context

`Scope::write_restricted` called `filesystem::write_regular_file`, which opened the destination with create and truncate, wrote, and flushed. Nothing synchronized the file or its directory, and the mode argument applied only to a newly created file. The audit's TLA+ model (`KeyFileCrash.tla` on branch `change/audit-finding-tla-models-20261115`) shows the in-place protocol violating both `WholeKeyAfterCrash` and `ReportedRotationDurable`, while the staged protocol (temporary file, sync, rename, directory sync) satisfies both.

## Decisions

### Decision: Stage, synchronize, then link or rename in the same directory

**Choice:** `publication::publish_regular_file` opens the destination's parent without following links, refuses a non-regular or symlinked destination, writes the payload to `.{leaf}.stage-{process}-{sequence}` created exclusively with the requested mode, synchronizes it, then either hard-links it to the leaf (no replacement) and removes the stage, or renames it over the leaf (replacement), and finally synchronizes the directory.

**Rationale:** a rename or link within one directory changes the leaf in one step, so a crash exposes either the previous state or the complete new file. The hard link gives no-replace creation with the capability-std API already used by node state, without adding a dependency to `molten-node-host`, whose production dependencies are fixed by the node-host boundary test.

### Decision: Secrets only; other writers keep their semantics

**Choice:** only `write_restricted` and the new `write_new_restricted` use staged publication. `Scope::write`, `DirectoryRoot::write` and `DirectoryRoot::write_new` are unchanged.

**Rationale:** `write_new` deliberately retains partial bytes for ingress reconciliation (audit F03 publication recovery), and enumerated stores such as the control inbox must not see stage names. Secrets are read by exact path, so a stage left by a crash is never mistaken for a key; the next publication of the same leaf removes it.

### Decision: Report post-commit failures as uncertain

**Choice:** if removing the stage after a link or synchronizing the directory fails, the error carries `FailureClass::UncertainExternalOutcome`.

**Rationale:** the new key is already visible, so the failure is neither success nor a clean rollback. A retried first boot observes and loads the existing key instead of generating another.

## Risks / Trade-offs

- Hard links must be supported by the filesystem holding node state; a filesystem without them fails first-key creation closed.
- Success observes payload and directory synchronization, not hardware durability; a concurrent publisher of the same leaf can make either publication fail, never produce a mixed file.
