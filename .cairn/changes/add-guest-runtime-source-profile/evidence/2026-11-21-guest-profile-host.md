# Guest runtime profile host evidence (2026-11-21)

## Output

`scripts/make-guest-runtime-binding` produced and signed:

```
/nix/store/qjb341yh9f1p9jzw4m3s49vx64g5x6pg-molten-guest-runtime-binding-v1
```

- producer profile: `molten-nix-guest-runtime-binding-v1`
- measured runtime paths: 10
- output NAR: 109,233,552 bytes
- input-addressed (`ca = null`), signed by `molten-guest-20261120`
- `binding.json` `dependencies_ref` binds the runtime-only `build-closure`

## Admission and driver

`molten node run` authenticated the output and published a startup receipt.
The real collaboration driver then passed:

```
test driver::collaboration_native_baseline ... ok
baseline observations=blake3:48ab2650cc296f88684f026d89265d6f5b0c253d40ab1b9251b6e21c605d9ddc
```

## Boundary

Host-only baseline: three processes on one host with an experimental runtime-only
signer. No KVM guest campaign is claimed. The profile does not prove the omitted
build, compiler, or gate closures.

## Guest bottleneck

The guest image reaches all three node setups, starts all three nodes, and then
enters `scope::select`. Each scope observation invokes `flow::authorize`
(two `molten node service-host` previews) plus the operation itself; every
process re-runs source authentication through the guest Nix daemon. Under
exact single-step, those invocations take minutes, so the driver does not finish
within the bounded run. Loopback was brought up explicitly and node setup now
completes; the remaining cost is repeated per-process authentication.

## Admission cache

The guest profile now records a source-gate admission in
`cache/source-gate-admission.json`. The key binds trust bytes, binding, executing
path and length, and measured artifacts; retained inputs and artifacts are still
revalidated on every hit. Full-profile admission never reads or writes it.

Host timing on one node root:

```
first  10.10s
second  0.03s
```

The second run reached the daemon restart check, confirming the cached gate was
accepted. Guest setup dropped from about 5:50 to 2:46 and all nodes were ready by
3:50; `scope::select` remains slow under exact single-step.
