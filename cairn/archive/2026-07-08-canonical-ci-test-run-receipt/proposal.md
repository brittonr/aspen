## Why

The Nix nextest check already preserves Cargo metadata, binaries metadata, and JUnit output. Those files are useful, but Molten's evidence model should bind the test-run decision in a canonical receipt that names the source, profile, binary metadata, rendered JUnit view, and pass or deny decision.

A CI test-run receipt makes CI evidence portable, comparable, and easier to reference from dogfood and release evidence bundles.

## What Changes

- Add a canonical CI test-run receipt for nextest-backed checks.
- Bind source ref, Cargo metadata ref, binaries metadata ref, nextest config ref, profile id, JUnit ref, test counts, and decision.
- Treat JUnit as a rendered view over the canonical receipt.
- Fail closed if required metadata or rendered artifacts are missing, stale, or mismatched.

## Impact

CI output becomes first-class Molten evidence rather than a collection of adjacent files.
