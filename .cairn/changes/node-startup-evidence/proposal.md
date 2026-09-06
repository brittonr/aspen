# Portable node startup evidence

## Why

Normal node startup previously manufactured an Octet test receipt. The new guard correctly rejects that path. The existing source-gate validator also reads workspace metadata from the current directory. A deployed node has no such workspace.

## What

Add a bounded, read-only verifier over an explicitly approved source/binary/tool cohort and a complete portable evidence bundle. Separate trusted operator expectations from untrusted evidence. Reuse the existing strict Octet policy with explicit metadata instead of a fabricated workspace.

Keep startup and the VM runner blocked until actual pinned-tool execution, portable verification, and lifecycle integration are evidenced. A verification-only report must not grant startup authority.

## Scope

Molten owns source-gate policy and its capability adapter. Octet owns findings and its artifact formats. Mantle/Onix retain package and VM admission. No dependency pin changes, compiler installation, Stage0, host blob listener, package rebuild, or release promotion is in scope.
