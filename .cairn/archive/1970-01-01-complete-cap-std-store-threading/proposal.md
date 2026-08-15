## Why

Aspen already depends on `cap-std`, exposes typed artifact, chunk, retention, dataspace, and exchange root aliases, and archived `adopt-cap-std-artifact-store-boundaries`. The production store APIs still accept ambient `&Path` roots and the targeted modules still call `std::fs` for reads, writes, enumeration, and deletion. The aliases therefore describe an intended boundary without carrying authority through the effectful operations that need it.

This is a correctness and review gap: path validation and a capability-root constructor do not provide `cap-std` containment when the subsequent I/O re-enters the ambient filesystem namespace.

## What Changes

- Make typed capability roots the required authority parameter for artifact, chunk, retention, local dataspace, and local exchange effects.
- Open operator-selected roots once in the CLI or runtime shell, then perform child operations relative to `cap_std::fs::Dir` without reconstructing ambient paths.
- Open Redb files through handles acquired beneath the capability root before passing them to Redb's file-backed builder.
- Replace path-returning enumeration and mutation helpers with validated relative locator and directory-entry APIs.
- Add a scoped structural regression gate that rejects new ambient filesystem calls inside converted store adapters while allowing explicit root acquisition in outer shells.

## Impact

- **Files**: `src/local_store.rs`, `src/artifacts/**`, `src/chunk/**`, `src/retention/**`, `src/remote/parts/dataspace/**`, `src/iroh/parts/exchange/**`, their CLI shells, authority-audit rules, tests, and local-filesystem documentation.
- **Testing**: positive in-root store workflows; negative traversal, absolute-path, symlink, locator-confusion, handle-substitution, and ambient-call fixtures; focused store and structural-audit checks.
- **Compatibility**: path-taking public shell entry points may remain temporarily as thin adapters, but they must acquire a capability and immediately delegate; effectful store internals no longer accept ambient roots.
- **Claims**: the change bounds local filesystem authority only. It does not prove durability, atomicity, artifact truth, confidentiality, remote transport correctness, or distributed-system correctness.
