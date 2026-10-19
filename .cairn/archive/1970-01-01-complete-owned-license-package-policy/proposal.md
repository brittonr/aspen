## Why

The accepted licensing boundary already requires AGPL-3.0-or-later for Molten-owned packages. All four workspace manifests select that expression, but license-policy.tsv lists only three packages: molten-node-host is omitted. The existing checker requires exactly one package rule per workspace member before evaluating the remaining rules, so the incomplete policy blocks the owned-source audit.

## What Changes

- Add the missing molten-node-host package rule without changing package grants or checker semantics.
- Clarify the accepted deterministic-validation requirement with complete workspace-rule coverage.
- Explain the complete workspace coverage and prospective owned-source boundary in the existing licensing documentation.

## Impact

- **Files**: license-policy.tsv, README.md, and the project licensing requirement delta in this lifecycle package.
- **Verification**: The parent performs the focused license-boundary command and final Cairn validation after source preparation. No builds, tests, lint, formatting, or broad validation are run by this worker.
- **Authority**: Earlier permissive grants, including byte-identical previously granted content, remain valid. License metadata is not publication, acquisition, or release authorization.

## Out of Scope

No AGENTS.md edits, publication, remote changes, dependency pins, credential URLs, lockfiles, generated build plans, frozen imports/fixtures, vendored material, or historical dependency-license assertions change. No claim of anonymous source acquisition or anonymous buildability is made.
