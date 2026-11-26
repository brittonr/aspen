# Steel sandbox and metering

Reviewed Steel actors run inside the harness as orchestration glue, not as an authority path. This article explains how the [Steel executor](../../../src/harness/steel/executor.rs) confines a script and bounds its resources. It assumes the hostcall boundary described in the [architecture](../../architecture.md). The governing requirement is `r[molten.bounded_execution.steel_sandbox_and_real_bounds]`; this [Technical companion](../README.md) article does not add to it.

## A closed language instead of a blocklist

A script is parsed with Steel's reader (`Engine::emit_ast`, which performs no module loading) and admitted only when every top-level form is a `define`. Bodies may use `lambda`, `if` with an explicit else branch, `begin`, `quote` over symbols, literals and proper lists, application, fixed-width integers, strings, and booleans. `require`, macros, `set!`, nested definitions, rest parameters, vectors, characters, and floats are refused before any VM exists.

Identifiers are checked lexically. A reference must name a parameter in scope, a top-level definition, or one of the sandbox primitives (`molten-hostcall`, `string-append`, `string-length`, `substring`, `string=?`, `equal?`, `not`, `+`, `-`, `*`, `=`, `<`). Every user binding is renamed into a private `molten-user-` namespace when the program text is rendered, so a script cannot spell a compiler keyword, a VM-internal global, or a primitive name by binding it. The callable is validated as a closed identifier, resolved as a top-level definition, and invoked by name with the input passed as a Steel string value; neither the callable nor the input is spliced into program text.

The VM is `Engine::new_raw_no_kernel()`: no prelude, no built-in modules, no kernel macro engine. Besides Steel's own private bootstrap globals (whose `#`/`%`-prefixed and module-loader names a script cannot spell), only the metered sandbox primitives are registered, so there is no filesystem, port, process, environment, network, clock, random, or thread primitive to reach. `Engine::new_sandboxed()` was not used: by source review of the pinned `steel-core` 0.8.2, its `register_builtin_modules` still registers the process and filesystem modules (nameable through `require-builtin`), and its prelude still exposes standard I/O, time, and threads; it also offers no execution metering.

## Real fuel, memory, and hostcall bounds

Rendering inserts a fuel-tick call at the start of every lambda body. All iteration in the admitted language is application, so every loop — including anonymous self-application such as `((lambda (x) (x x)) (lambda (x) (x x)))` — spends one unit of fuel per entry. Each primitive call and each closure creation spends one unit as well. Fuel exhaustion is deterministic: the same script and input stop at the same point with the same meter.

Primitives are registered only under private `molten-prim-` names, and the renderer maps each public name (`+`, `string-append`, …) to its private global. Steel's compiler therefore never recognizes a call such as `(+ local 1)` as a built-in opcode; every call reaches the metered, overflow-checked primitive.

Every primitive charges the bytes it produces, and the input is charged before the call. Every lambda expression is rendered inside a closure-charging call, so each closure evaluation charges a base cost plus one slot per variable it can capture (everything lexically in scope) or bind; the renderer knows these counts statically. A single produced string may not exceed 8 KiB and cumulative charged allocation may not exceed 64 KiB, so string doubling and closure-chain growth stop on the memory ceiling long before fuel runs out. Lambdas take at most 16 parameters and applications pass at most 16 arguments, so each call frame has a bounded size and live frames are bounded by fuel.

The VM runs on a dedicated thread with a 64 MiB stack, so the engine, every value it built, and Steel's recursive drop of closure chains never consume the caller's stack; the charged closure ceiling bounds any chain to a few thousand links.

`molten-hostcall` admits each call against `STEEL_MAX_HOSTCALLS` before it produces a response. The call past the limit is refused and never dispatched. Exhaustion of any ceiling is sticky — every later metered operation fails — and also interrupts the VM through its thread-state controller, so no script path continues past a ceiling. The execution receipt reports the real remaining fuel and admitted hostcall count.

## Resolved discrepancy

Earlier revisions ran scripts in `Engine::new()` with the full standard library, estimated fuel from source and input length without runtime metering, relied on a fourteen-token substring blocklist that anonymous lambda recursion and string doubling bypassed, counted hostcalls only after the script finished, and interpolated the callable name into source text. The [sandbox tests](../../../src/harness/steel/tests.rs) cover ambient access, macro and mutation forms, keyword shadowing, anonymous recursion, string doubling and cumulative allocation, closure-chain and closure-count growth (from a 2 MiB caller thread), arithmetic on locals with exact fuel accounting and overflow, arity caps, hostcall flooding, and callable injection, together with a legitimate script whose metering is deterministic.

## Limits and non-claims

The bounds are logical: fuel counts lambda entries, closure creations, and primitive calls, not VM instructions, and memory counts primitive and closure allocation, not the allocator's resident set or the VM's bounded per-frame stack. The harness preflight's ambient-token list in the suite schema remains a review aid; it is not the sandbox boundary. The sandbox does not establish operating-system isolation, and a Steel receipt does not certify script correctness.

## Sources

- [Steel executor](../../../src/harness/steel/executor.rs)
- [Steel sandbox tests](../../../src/harness/steel/tests.rs)
- [Architecture: Steel orchestration](../../architecture.md)
- [Technical companion](../README.md)
