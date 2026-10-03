# Inspecting capture and restore plans

Mode: How-to

## Goal and prerequisites

Decide whether supplied capture evidence supports planning a restore, and identify the next boundary without activating a runtime. You need an exact world-commit reference from retained evidence, access to its existing capability-rooted state store, and a fresh output location. For execution-snapshot compatibility you additionally need canonical source and destination descriptors, not merely a commit reference.

This is a source-checked procedure, not an executed restore tutorial. The standalone surfaces do not supply capture or runtime restore adapters. Start from the [Handbook](../README.md); use [typed roots and capture](../../technical/world-state/typed-world-roots-and-capture.md) for theory rather than treating a plan as admission.

## 1. Decide which artifact you actually have

A workflow checkpoint plan means the workflow planner accepted planning facts. A capture receipt concerns the capture shell's observations and publication outcome. A world commit identifies immutable profile-relative roots. A snapshot descriptor describes an execution restore mechanism and cohort. None substitutes for the other three.

If you have only the logical workflow fixture, stop before local-store inspection: its references are fixture identifiers, not proof that your state root contains those objects. Request the actual capture evidence and store location from the operation owner. Do not initialize or populate a production store merely to make an inspection command succeed.

## 2. Inspect the capture boundary first

There is no `world-commit capture` subcommand in the inspected declaration. For an embedding, trace `capture_world_commit` in the [capture shell](../../../src/worldcommit/shell.rs): preflight and observation precede pure planning; missing roots are persisted; durable bytes are verified; revision and inventory fences are rechecked; commit publication happens last.

Ask the owner for the capture receipt and the exact immutable commit, together with adapter evidence for observed revisions and inventory completeness. The shell returns no successful commit on drift or uncertain publication. Root files left by an interrupted capture do not demonstrate that the final coherent cut was published. Do not delete those files as an incident-repair shortcut.

## 3. Inspect identity and profile before closure

Set `WORLD_STATE_ROOT` to the existing reviewed state root and `WORLD_COMMIT_REF` to its exact evidence-backed reference. These guards prevent empty substitutions, not incorrect operator choices. The following command is source-checked and not executed here; provenance is the [commit declaration and loader](../../../src/cli/runtime/worldcommit.rs), [main alias](../../../src/main.rs), and [top-level command registration](../../../src/main/root/parts/command/p000/body.rs).

```sh
molten world-commit inspect \
  --state-root "${WORLD_STATE_ROOT:?Set the existing reviewed state root}" \
  "${WORLD_COMMIT_REF:?Set the exact captured commit reference}"
```

The loader parses canonical commit bytes against the requested reference. The diagnostic view identifies profile and roots. Check logical versus opaque versus mixed completeness against the [world-commit root table](../../world-commit.md#typed-roots). Historical authority observations are not current permissions. Logical capture excludes an opaque machine root; opaque and mixed commit profiles require their exact cohort reference.

## 4. Validate closure, then request a plan

Use distinct fresh output paths and continue to planning only after reviewing successful closure validation. Both commands below are source-checked, not executed; their positional commit argument, optional `--out`, and behavior are declared in [worldcommit.rs](../../../src/cli/runtime/worldcommit.rs).

```sh
molten world-commit validate \
  --state-root "${WORLD_STATE_ROOT:?}" "${WORLD_COMMIT_REF:?}" \
  --out "${WORLD_CLOSURE_OUT:?Set a fresh closure-report path}"
```

```sh
molten world-commit plan-restore \
  --state-root "${WORLD_STATE_ROOT:?}" "${WORLD_COMMIT_REF:?}" \
  --out "${WORLD_RESTORE_PLAN_OUT:?Set a fresh restore-plan path}"
```

An incomplete closure can still produce its report before returning an error. Preserve that report. The restore-plan path instead rejects incomplete closure before writing a successful plan. Closure establishes bounded declared object presence and identity, not successful restoration or current admission.

The CLI maps successful local root readback to both identity and schema-match observations. That is the inspected adapter boundary, not evidence that an arbitrary subsystem schema validator ran. Review schema meaning with the responsible component rather than broadening the report's claim.

## 5. Separate restore planning from execution compatibility

Execution-snapshot planning is a distinct command family. Its `--destination` argument is another descriptor file, whose cohort is used for comparison; it is not a destination directory or host discovery mechanism. Its `--current-admission` flag supplies a planning boolean, not a live authority adapter. Do not set it just to get a successful plan.

The [snapshot CLI](../../../src/cli/runtime/worldsnapshot.rs) denies standalone restore with an adapter-required receipt. The [snapshot contract](../../world-execution-snapshots.md) requires exact opaque cohort matching, new host handles, and current admission before activation. A commit restore plan cannot discharge those requirements.

## Worked failure and stop decision

Suppose captured task bytes exist and match their reference, but the task revision changes before the capture fence recheck. The capture shell denies before commit publication even though root persistence succeeded. A later closure report over some other complete commit does not repair that failed capture. Preserve the denied receipt, identify the mutable source and inventory evidence, and arrange a new reviewed capture through the owning embedding only after the observations are understood. If publication was uncertain instead, stop for reconciliation rather than assuming a new capture is a harmless retry.

## Sources

- [Handbook](../README.md)
- [World-commit contract](../../world-commit.md)
- [Execution-snapshot contract](../../world-execution-snapshots.md)
- [Typed roots and capture companion](../../technical/world-state/typed-world-roots-and-capture.md)
- [Capture and restore shell](../../../src/worldcommit/shell.rs)
- [Commit CLI](../../../src/cli/runtime/worldcommit.rs)
- [Snapshot CLI](../../../src/cli/runtime/worldsnapshot.rs)
