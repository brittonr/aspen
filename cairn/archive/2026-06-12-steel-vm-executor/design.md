# Design: Steel VM executor

## Boundary

Steel execution is enabled only for actor registry entries carrying a reviewed Steel executor fixture and matching review receipt. The fixture binds:

- source/module ref;
- callable name;
- allowed hostcalls;
- Steel engine/version profile;
- sandbox/resource profile;
- executor conformance suite refs.

The VM is created with no ambient authority. It receives input only as a canonical Preserves value and can affect Molten state only by returning canonical actor-output values or by invoking registered hostcall primitives.

## Preserves bridge

The runtime converts the same `<actor-input-v1 ...>` envelope used by native and Wasm actors into a Steel value through a deterministic Preserves bridge. The callable returns either:

- a canonical actor-output Preserves value; or
- a typed error value that becomes a canonical executor failure artifact.

The bridge must not coerce booleans, symbols, records, byte strings, or refs into lossy host-language strings. Canonical Preserves refs for input and output are included in the Steel execution receipt.

## Hostcall primitives

Only reviewed hostcall primitives are injected into the VM. Each primitive builds a canonical `<hostcall-request-v1 ...>` envelope and submits it to the runtime shell. The shell performs policy/capability/budget/effect-log checks and returns a canonical decision/response value to Steel.

Steel source cannot access raw filesystem, network, process, clock, random, environment, dynamic eval/load, or unreviewed modules. Static source scanning remains a preflight guard, but the VM sandbox and hostcall registry are the enforcement boundary.

## Determinism and resources

The executor should expose deterministic limits for:

- reduction/instruction/fuel count;
- allocation/heap size;
- maximum hostcall count;
- maximum input/output byte size;
- recursion/stack limits when available.

Timeouts based on wall-clock time are diagnostic only unless converted into deterministic fuel/resource exhaustion. Any resource exhaustion fails closed before runtime state commit.

## Evidence

`<steel-execution-receipt-v1 ...>` binds source ref, callable, review receipt ref, engine profile ref, sandbox ref, input ref, output ref, hostcall refs, resource limits/usage, and checks for reviewed callable, no ambient authority, canonical Preserves bridge, hostcall admission, and replay binding.

Replay re-runs the callable with the same input and recorded effect/hostcall responses, then compares output and hostcall refs exactly.
