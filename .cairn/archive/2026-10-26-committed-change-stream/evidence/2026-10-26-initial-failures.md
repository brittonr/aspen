# Subscriber conformance: retained initial failures

## Pagination regression before the fix

Command (repository Nix shell):

```sh
cargo test -p molten-core --lib -- committed_stream world_faults
```

Result: **18 passed, 1 failed**. The new
`bounded_pages_do_not_skip_events_within_one_committed_revision` regression
received ordinal 2 when it expected ordinal 1 on its second one-event page:

```text
left:  event_ref=blake3:96f0f64298b2b930d39ae9810ace55ee4d024c88a9e79fae09eaf7aa1bbe6809, revision=1, ordinal=2
right: event_ref=blake3:5528e9e08a7a01f5180c7bd1ca4b73b7603f90f00e3ce9d7e7745f22e5465575, revision=1, ordinal=1
```

`StreamCursor::after` already advances the ordinal. The old strictly-greater
comparison skipped that next ordinal. The fix makes subscription inclusive of
the next-event cursor, preserving the existing event/cursor identity format.

Pre-change root `cargo test -p molten --lib -- world_faults`: **4 passed**.

## Focused source gate during integration

The first local `world-faults-octet-deny-all` run denied 39 diagnostics (several
are producer duplicates): platform-dependent casts, integer fallback sentinels,
unbounded duplicate accumulation, imported concrete collection types, public
`usize`, boolean naming, wildcard enum matching and one model directory exceeding
its file-count bound. These findings were repaired in source, without new lint
allows or policy exemptions. The final focused check reports zero findings.

The remote-builder attempt did not exercise the source check: Nix could not open
a build-farm lock pathname (`File name too long`, then `unexpected EOF`). The
successful bounded Nix checks ran with `--builders ''`; no builder configuration
or dependency pins changed.

## Environment-only smoke invocation

An evaluation-kernel subprocess could not find `cargo` inside its inherited
shell environment. It did not run the scenario. Execution through the regular
repository `nix develop` shell succeeded. A separate attempt to parse a truncated
reader rendering of the generated JSON was discarded; the final identity check
uses the exact `rust_profile_ref` field. Neither failed invocation is counted as
passing evidence.

The existing kache index I/O warning disabled caching, not compilation. No cache,
unrelated process, filesystem, or repository state was deleted to mask a failure.
