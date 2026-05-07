# MicroVM runtime architecture docs

- Change: `implement-microvm-runtime-runner`
- Task: update runtime architecture docs/source anchors
- Started: `2026-05-07T02:56:29Z`
- Completed: `2026-05-07T02:57:18Z`

## Documentation updates

- Updated `docs/runtime-applications.md` to define the portable microVM runner/profile boundary.
- Added operator-facing anchors for:
  - `MicroVmRuntimeProfile`;
  - `MicroVmEngine`;
  - Firecracker, Cloud Hypervisor, Uhyve, and QEMU microvm engines;
  - virtualization backend reporting;
  - runner capability/version matching;
  - supported guest artifact profiles;
  - `LinuxGuest` and `Unikernel` artifact identities;
  - declared launch bindings;
  - lease/heartbeat state;
  - `RuntimeHostKind::MicroVm` admission;
  - mount/block/network/vsock/metadata/capability/output binding denial;
  - denial of ambient host paths, devices, sockets, networks, secrets, and undeclared handles before boot.

## Verification

- Source-anchor assertion over `docs/runtime-applications.md` printed `microvm runtime docs anchors present`.
- `openspec validate implement-microvm-runtime-runner --strict`
- `git diff --check`
