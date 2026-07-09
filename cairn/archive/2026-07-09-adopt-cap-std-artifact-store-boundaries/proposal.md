## Why

Aspen's local chunk, retention, dataspace, and exchange paths manipulate stored artifacts under local roots while also handling remote or manifest-provided locators. Ambient filesystem operations make it too easy for a bad locator to become a path traversal or symlink escape review problem.

## What Changes

- Adopt `cap-std` at Aspen shell/adaptor boundaries that open local artifact, chunk, retention, dataspace, or exchange roots.
- Pass typed capability roots into local store adapters while keeping manifest, catalog, and identity cores filesystem-neutral.
- Replace hand-rolled containment checks in targeted store paths with capability-relative operations.
- Add positive and negative fixtures for valid in-root chunks, `../`, absolute paths, symlink escapes, and remote-locator misuse.

## Impact

- Local artifact-store authority becomes explicit and reviewable.
- Canonical chunk, manifest, and content identity semantics remain unchanged.
- Invalid locators deny before exposing or mutating local artifact bytes.
