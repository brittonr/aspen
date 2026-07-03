## Why

Molten has broad test coverage and a large accepted Cairn spec surface, but review still requires manual inspection to answer which requirements have positive and negative verification. A requirement-to-test traceability gate makes evidence gaps visible when requirements change and prevents broad release claims from depending on untested normative text.

## What Changes

- Generate a traceability manifest from accepted `r[...]` requirements and verification markers.
- Record positive tests, negative tests, validation commands, evidence artifacts, and documented exemptions for each covered requirement.
- Fail or mark non-pass when evidence-bearing requirements lack required positive and negative coverage.
- Detect stale test references and requirement ids that no longer exist.
- Emit a compact operator summary for release review and local development.

## Impact

- **Files**: test harness/reporting code, Cairn/Tracey integration helpers or scripts, Nix checks, docs/README testing section.
- **Testing**: positive manifest fixtures, negative fixtures for missing coverage and stale refs, and an explicit Nix/Cairn gate that exercises the traceability report.
