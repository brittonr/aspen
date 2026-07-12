# Filesystem materialization authority

Molten treats bundle export, bundle unpacking, retention candidate handoff, and
release archive construction as filesystem materialization. A logical bundle
name is not filesystem authority. Reusable materializers therefore receive one
explicit destination capability and operate only through
`materialization::MaterializationRoot`.

## Boundary

Reviewed CLI shells may acquire a capability for an operator-selected input
file, output file, source directory, or destination directory. After that
bootstrap, child reads, directory creation, staged writes, verification,
publication, replacement, and rollback are capability-relative. Reusable code
must not reconstruct ambient descendant paths or call generic archive
`unpack` APIs.

Test-only adversarial setup may use ambient APIs to create symlinks and mutate
fixtures. That setup is not canonical materialization evidence.

## Pure plan

`MaterializationPath` accepts only non-empty UTF-8 relative paths made from
normal components. It rejects absolute and platform-prefixed paths, `.` and
`..`, backslash aliases, NUL bytes, empty components, trailing separators, and
reserved internal staging names.

`MaterializationPolicy` bounds member count, member size, total bytes, and path
length beneath process-wide hard safety ceilings, and states either `no-replace` or `replace-regular-files`. The pure
planner validates all members before mutation, rejects duplicate logical
paths, sorts members by canonical path, and computes a BLAKE3-backed Preserves
plan identity. The identity binds the profile, replacement policy, bounds,
logical path, member kind, content reference, and size. Host paths and ambient
root names are not included.

## Stage, verify, and publish

A destination root stages regular files beneath
`.molten-materialize/<plan-hash>/tree` inside the supplied capability. Every
staged file is created without following links and is reread to verify its
size and BLAKE3 content reference. Commit rejects a stage from another root or
a stale plan identity, rechecks destination components for links or special
files, and applies the declared replacement policy.

Replacement is limited to existing regular files. Existing files move to an
in-root backup before staged publication. Publication uses an in-root hard
link from the verified stage so a newly appearing destination cannot be
silently clobbered, then removes the staged link. A failed publication or post-write
verification rolls published files back and restores backups; it does not mint
a passing receipt. Successful publication verifies every destination member
before deleting quarantine state and constructing the receipt.

`no-replace` is the default. Replacement must be selected by a named workflow
profile and cannot replace links, directories, or special files.

## Archive members

Shared archive writing and verification reuse the materialization path and
bound policy. Writers emit sorted regular files with deterministic metadata.
Readers stream each member through configured limits and reject duplicate,
absolute, traversal, link, device, directory, and other special entries. A tar
header name is never passed to a generic extraction API. Verified archive
payloads may then be supplied to the same destination planner and staged
publisher.

## Receipts and claim boundary

A passing `molten.filesystem-materialization-receipt.v1` records the plan
identity, profile, destination authority kind, replacement policy, ordered
member evidence, total bytes, checks, and explicit non-claims. Receipt
validation recomputes both the ordered member set and plan identity.

The receipt proves only that the listed bytes were verified and published
under the supplied capability according to the recorded policy. It is not a
proof of artifact semantics, archive provenance, source trust, authorization
to disclose payloads, concurrent adversarial race freedom, distributed
atomicity, crash consistency, durability, or release readiness.

## Regression gate

`tools/ast-grep/runtime-authority/rules/materialization-ambient-output.yml` is
a blocking structural check for converted repro, retention, and release
materializers. Its positive fixture demonstrates forbidden ambient descendant
I/O and generic archive unpacking; its negative fixture demonstrates the
capability shell. Findings invalidate the runtime-authority audit receipt but
remain syntactic evidence only.
