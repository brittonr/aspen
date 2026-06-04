# Change: steel-vm-executor

## Why

Molten now has reviewed Steel executor preflight evidence, but Steel actors still do not execute in a Steel VM. Steel is intended for reviewed dynamic predicates and trusted callables, not as a hidden policy/config language. The next slice needs an execution path that preserves the existing Nickel/Basalt boundary and routes every dynamic effect through canonical hostcall evidence.

## What

- Add a reviewed Steel VM executor path for actor callables whose source/callable/allowed-hostcall review receipts validate.
- Pass canonical Preserves actor-input values into the reviewed callable and require canonical Preserves actor-output values back.
- Expose only Molten hostcall primitives that produce canonical hostcall request envelopes and receive admitted decisions/responses from the runtime shell.
- Disable ambient filesystem, network, process, clock, random, dynamic load, and unreviewed module access.
- Bind Steel engine version, source ref, callable, review receipt, input/output refs, hostcall refs, and resource limits into Steel execution receipts.
- Add conformance and negative suites parallel to the Wasm executor path.

## Impact

Reviewed Steel can become a real dynamic predicate/trusted-callable executor while maintaining fail-closed policy, capability, budget, replay, and evidence gates. Nickel remains the static declarative policy/config boundary; Steel execution is allowed only with explicit review and executable receipts.
