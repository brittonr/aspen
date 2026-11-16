## Context

`THIRD_PARTY_LICENSES.md` mixes current resolved dependencies (`Cargo packages ...` rows), copied historical fixtures, vendored snapshots and external references. The Mantle row selects `dd2beb…` while a historical fixture row keeps `cf7623…`; a whole-file search cannot tell them apart. The checker is a std-only Rust script, so `rustc --edition=2024` builds it without Cargo, as the `inherited-tracey-debt` check already does for its guards.

Documentation entry pages link about 3,700 local targets. The existing `atlas-current` and `evidence-publication` checks show the pattern for stdlib Python checks over `sourceForConfigChecks`. Several non-VM checks (for example the `nixos-vm-*` shard checks) already put the `molten` package on `PATH`, so a quick-start check reuses a build that CI already performs.

## Decisions

### Decision: Bind a notice revision to the row that names the dependency

**Choice:** A `notice-revision` rule selects the single Markdown table row whose first cell contains `Cargo packages` and the exact backticked dependency name, and requires `revision \`<selected>\`` in that row. Zero or several such rows fail. Every declaration of the dependency in the workspace manifest must select the same revision.

**Rationale:** The row subject already classifies current resolved dependencies, so no second classification file is needed. Historical fixture, snapshot and reference rows never name a package as `Cargo packages`, so their older revisions neither satisfy nor fail a rule. Requiring agreement across declarations prevents a dev-dependency or target-specific entry from drifting silently.

**Alternative rejected:** comparing against `flake.nix` inputs too. `scripts/git-source-hashes.sh --check` and the release-dependency profile already bind Cargo and Nix source identities; the notice rule only needs the Cargo selection.

### Decision: Run the license checker as a flake check

**Choice:** `license-boundary` compiles the checker with the pinned toolchain and runs it on the source tree; the existing `dependency-audit` app keeps calling it.

**Rationale:** The rule previously ran only in a manually invoked app, which is how a renamed documentation marker went unnoticed.

### Decision: A bounded link checker, not a crawler

**Choice:** Check inline links, images, reference definitions and HTML `href`/`src` outside fenced code and code spans. Skip any target with a URL scheme or network path. Paths must exist inside the repository. Fragments must match GitHub heading slugs or explicit `id`/`name` attributes for Markdown, `id` attributes for HTML, or `#Ln[-Lm]` line anchors within the line count for other files; any other fragment fails rather than passing unchecked.

**Rationale:** External URLs make CI depend on the network. The checked set covers the public entry points named in #29 without reviewing archived material; archives are read only as link targets.

**Trade-off:** Links into an active change (the atlas tracking link and the private-reporting observation) break when that change is archived. The check then fails and names the link, so the archiving change must repoint it; this is the drift signal the check exists to give.

### Decision: Run the README quick start with the packaged binary

**Choice:** Extract the "Getting started" fenced shell blocks that invoke `cargo run --locked --` or remove the demo directory, substitute `"$MOLTEN_BIN"`, run them with `set -euo pipefail` in a fresh directory, and require the three results the README states (and that the README still states them). The clone and `nix develop` block is not run.

**Rationale:** The commands, arguments and promised output stay bound to the prose without a sandboxed Cargo build. The clone/shell block needs the network; the hosted reproducible-dependencies workflow owns it. A pass is not a verbatim cold `cargo run` execution (#25 AC4).

### Decision: Ground atlas payloads in source types

**Choice:** Each edge carries `payload = { type, fields, source }` naming an existing type and its fields. Answered questions need `tracking` entries linking an issue or Cairn change. The HTML twin shows the payload on hover or keyboard focus, and a pin button (one small inline script) keeps it open.

**Rationale:** Naming real types keeps payloads checkable by review. While grounding them, no world module was found to use the Prolly map (the Prolly document calls the map product-local), so the `world → prolly` edge is replaced by `world → node-state`, carrying `CaptureReceipt`, and the `world` source path is corrected to `src/worldcommit/`.

### Decision: Record what the private-reporting read does and does not show

**Choice:** Retain one dated read-only API observation in this change's evidence, and state in SECURITY.md and the tracker that it shows an enabled setting, not report delivery or maintainer handling.

**Rationale:** #30 asks to verify the channel before documenting it. Submitting a test report would create an external side effect and is out of scope; the observation is the reachable part.

## Risks / Trade-offs

- The slug algorithm approximates GitHub's renderer; unusual heading markup may need an explicit anchor.
- The quick-start check depends on the full `molten` package build. Existing checks already require that build, so it adds a run, not a build.
- Atlas payload field lists are curated subsets; they are reviewed against source, not generated from it.
