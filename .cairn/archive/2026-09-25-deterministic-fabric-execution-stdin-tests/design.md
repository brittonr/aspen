# Design: Make the fabric_execution stdin tests deterministic

## Context

`LiveExecutionAdapter` (`src/fabric_execution/live.rs`) calls `bounded_exec::run`. It maps every post-spawn `RunError` to
`UnknownAfterStart`. At the pinned revision, `bounded-exec`'s `finish_stdin_worker` ignores `BrokenPipe`,
`ConnectionReset`, and `ConnectionAborted` only when completion is not `Exited`. After a normal exit, the stdin error
discards the collected exit status and output. Whether a non-reading child exits before a 6-byte write therefore
depends on scheduling.

## Decisions

### Decision: Remove input from non-reading children instead of changing classification

**Choice:** The fixtures give no input to children that do not read it (`request_without_input`,
`canonical_request_without_input`, `resolved(None)`). The one child that must receive input reads it with the builtin
`read`.

**Rationale:** The tests' subjects are exit policy, publication failure, teardown, and composition shape. Unread input
added a scheduling race that is unrelated to those subjects. Changing adapter classification here would guess at a
`bounded-exec` contract that does not exist yet.

### Decision: Pin the conservative mapping with an oversized input

**Choice:** A dedicated test sends `PIPE_OVERFLOW_INPUT_BYTES` (256 KiB) to `exit 0`, using a descriptor and request
whose stdin bound admits that size.

**Rationale:** A 6-byte write races the child's exit. A write larger than the pipe capacity cannot complete once the
child has exited, so `EPIPE` is certain and the test is deterministic. The assertions pin the uncertainty requirement
(`molten.fabric_execution.uncertainty` in `.cairn/specs/bounded-execution/spec.md`): unknown, no completion claim, and no publication. The follow-up that adopts
`InputDelivery::ClosedByChild` revises this test.

### Decision: Builtin `read` instead of an absolute program path

**Choice:** The composition child uses `IFS= read -r value; printf 'bounded:%s' "$value"`.

**Rationale:** No absolute `cat` path exists in both the Nix sandbox and developer hosts. The builtin needs no `PATH`,
and it blocks until the write has happened, so the 6-byte write always completes first.

## No-spec classification

Accepted requirement text does not change. Semantic review inputs are the three test files, `src/fabric_execution/live.rs`
(unchanged mapping), and `.cairn/specs/bounded-execution/spec.md` (`molten.fabric_execution.uncertainty`).

## Risks / Trade-offs

- The pinning test depends on the default Linux pipe capacity (64 KiB) staying below 256 KiB. A host that raised the
  default pipe size above 256 KiB would buffer the input and turn the case into a normal exit.
- Non-reading children no longer exercise the stdin writer. The round-trip test and the pinning test still cover it.
