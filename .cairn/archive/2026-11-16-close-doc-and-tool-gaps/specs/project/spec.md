# Project Specification Delta

## ADDED Requirements

### Requirement: Selected dependency revisions bind to their own notice rows
r[molten.project.license_boundary.notice_revisions] For every dependency named by a `notice-revision` license-policy rule, the repository check MUST require that all workspace-manifest declarations of the dependency select one revision and that this revision appears in the single `Cargo packages` row of `THIRD_PARTY_LICENSES.md` naming the dependency. A revision recorded only by another row (a copied historical fixture, vendored snapshot or external reference) MUST NOT satisfy the rule, and such a row MAY keep an older revision.

#### Scenario: A stale current-dependency row fails even when the revision appears elsewhere
- GIVEN the manifest selects hardened Wasmtime `57c834…` and Mantle `dd2beb…`
- AND their notice rows record `cf4a19…` and `cf7623…` while other text in the file names the selected revisions
- WHEN the license boundary check runs
- THEN it fails for each stale row.

#### Scenario: A historical fixture keeps its older revision
- GIVEN the current Mantle row records the selected revision
- AND an unchanged historical fixture row records the older Mantle revision `cf7623…`
- WHEN the license boundary check runs
- THEN it passes, and the historical row cannot stand in for a missing current row.

### Requirement: Public documentation links resolve locally
r[molten.project.documentation_links.local_targets] The repository check MUST reject a local link or reference definition in `README.md`, `CONTRIBUTING.md`, `SECURITY.md` or `docs/**/*.md`, outside code, whose target does not exist inside the repository, or whose fragment is not a heading slug or explicit anchor of a Markdown or HTML target, or a line anchor within the line count of another file. Links with a URL scheme MUST be skipped, not fetched. A passing check MUST NOT be presented as semantic review of the linked claims.

#### Scenario: A broken link is rejected
- GIVEN a documentation page linking a missing file, a missing anchor and a line past the end of a file
- WHEN the documentation link check runs
- THEN it fails and names the file, line and target of each.

#### Scenario: Valid and external links pass
- GIVEN a page with relative, root-relative, anchored, line-anchored and external links, and link syntax inside code
- WHEN the documentation link check runs
- THEN it passes without fetching the external targets.

### Requirement: README quick-start commands run with the packaged binary
r[molten.project.documentation_links.quickstart_commands] The repository check MUST run the README "Getting started" fixture and cleanup commands with the packaged `molten` binary in place of `cargo run --locked --`, in a fresh directory, and MUST fail if a command fails, if its output lacks a result the README states, or if the README no longer states that result. The check MUST NOT be described as a verbatim cold `cargo run` execution.

#### Scenario: A changed subcommand fails
- GIVEN the README names a subcommand the binary does not accept
- WHEN the quick-start check runs
- THEN it fails with the command's exit status.

#### Scenario: A missing documented result fails
- GIVEN the binary reports a status other than the documented `phase=stopped`
- WHEN the quick-start check runs
- THEN it fails naming the missing result.

### Requirement: Atlas edges name payloads and answered questions name their tracking record
r[molten.project.system_atlas.payloads_and_tracking] The system atlas generator MUST reject an edge without a payload type, non-empty field list and source path, and an answered question without at least one linked issue or Cairn change. Both generated twins MUST show each edge payload, and the HTML twin MUST reveal it on hover or keyboard focus and allow pinning it open.

#### Scenario: An answered question without tracking is rejected
- GIVEN an atlas question with status `answered` and no tracking link
- WHEN the atlas is generated
- THEN generation fails naming the question
- AND the same question with status `open` is accepted.
