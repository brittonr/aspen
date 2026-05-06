# Hermit runtime reference review

Status: captured

## Inputs

- https://github.com/hermit-os/uhyve
- https://github.com/hermit-os/hermit-no-std
- https://github.com/hermit-os/hermit-rs
- https://github.com/hermit-os/loader

## Findings

- `hermit-rs` describes Hermit as a Rust-based lightweight unikernel where the application is bundled with the kernel library and runs without an installed guest OS.
- `uhyve` is a minimal, special-purpose hypervisor for the Hermit kernel. On Linux it depends on KVM; it runs a Hermit unikernel image directly and supports bounded runtime parameters such as memory and CPU count.
- `loader` is a Hermit bootloader for several environments and targets, including x86_64 Linux, multiboot, UEFI, AArch64, RISC-V, QEMU, and QEMU's `microvm` machine type.
- `hermit-no-std` demonstrates a no-std Hermit application and boots it with the Hermit loader plus QEMU `-kernel <loader>` and `-initrd <app>`.

## Aspen design consequences

- Hermit should remain `RuntimeArtifact::Unikernel { unikernel_kind: HermitOs, image_hash }`, not a native process and not an OCI/Linux compatibility workload.
- `RuntimeHostKind::MicroVm` needs engine variants beyond Firecracker/Cloud Hypervisor: `Uhyve` for the Hermit-specific hypervisor and `QemuMicrovm` for QEMU microvm/loader development or test paths.
- Hermit manifests should distinguish the guest application image from loader/hypervisor/boot-profile artifacts. Receipts should record selected engine and artifact hashes, not mutable host paths, raw kernel args, or environment secrets.

## Verification IDs

- r[runtime-host-loading.host-taxonomy.microvm]
- r[runtime-host-loading.host-taxonomy.unikernel]
- r[runtime-host-loading.dynamic-artifacts.hermit-guest-verified]
