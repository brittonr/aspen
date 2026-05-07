# Hermit runtime architecture docs

- Change: `implement-hermit-unikernel-profile`
- Task: update runtime architecture documentation/source anchors
- Started: `2026-05-07T02:21:30Z`
- Completed: `2026-05-07T02:22:11Z`

## Updates

- Extended `docs/runtime-applications.md` runtime host-loading taxonomy with the public Hermit terms introduced in `aspen-runtime-core`.
- Documented `HermitUnikernelArtifact` as a guest-artifact profile that records application image, target architecture, guest ABI, immutable hash, launch profile, declared input channels, and redacted boot/serial evidence.
- Documented Uhyve and loader/QEMU compatibility mapping without claiming a concrete boot runner.
- Re-stated the non-goal: Hermit is not an OCI container, native process, or ambient host execution path.

## Verification

- Python docs anchor assertion for `HermitUnikernelArtifact`, `Uhyve`, `loader/QEMU`, microVM engine mappings, declared input channels, and non-OCI/native-process language.
- `openspec validate implement-hermit-unikernel-profile --strict`
- `git diff --check`
