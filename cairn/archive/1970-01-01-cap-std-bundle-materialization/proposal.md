## Why

Aspen materializes repro bundles, retention review bundles, dogfood release evidence, and other multi-file operator artifacts by joining an output `Path` with member names and calling ambient filesystem APIs. Several surfaces validate canonical refs well, but local output containment, pre-existing symlink behavior, duplicate logical paths, partial-write cleanup, and archive member path policy are implemented separately or left implicit.

`cap-std` is most useful here as a reusable output-directory authority: the shell can open one explicitly requested destination, a pure planner can validate logical members, and every write or rename can remain beneath that destination even when local namespace entries are hostile.

## What Changes

- Add a pure bounded materialization plan over logical relative member paths, content refs, sizes, replacement policy, and final manifest identity.
- Add a capability-rooted materialization shell for directories, files, staged commits, verification reads, and cleanup.
- Migrate repro export/unpack, retention candidate bundles, release evidence directories, and dogfood archive inputs/outputs to the shared boundary.
- Apply one archive-member path policy to tar read/write metadata even when a command only verifies and never extracts an archive.
- Add positive and negative tests for traversal, absolute paths, platform prefixes, duplicate members, symlink parents/leaves, wrong roots, partial failure, and stale plan reuse.

## Impact

- **Files**: a new materialization core/shell, `src/cli/runtime/repro/**`, retention bundle code, operator/dogfood release export code, archive helpers, structural authority rules, tests, and operator documentation.
- **Testing**: deterministic plan tests, capability-root integration tests, archive-member fixtures, interrupted/stale-plan tests, and existing bundle round trips.
- **Sequencing**: depends on `complete-cap-std-store-threading` to reuse relative-locator and capability-shell conventions.
- **Claims**: containment and manifest verification do not prove archive safety in unrelated tools, confidentiality, authenticity, signature validity, authority, release eligibility, or crash-atomic persistence.
