# Design

## Owner and boundaries

Cargo owns package-ID syntax and formatting. Molten owns selection of its development toolchain.
The patch changes only deterministic URL/name/version decisions in Cargo's schema crate.
Nix owns source fetches, patch application, compilation, and composition with the existing Rust tools.
No new port, service, wrapper that rewrites metadata, or runtime dependency is needed.

A targeted Onix Core search found no existing pathless package-ID patch for reuse.
The upstream Cargo component remains the owner. Its MIT/Apache and third-party notices remain intact.
The source revision is immutable. Git requires its native revision identity, while new evidence uses BLAKE3.

## Decisions

When a URL has no final path component, formatting writes the explicit package name.
Parsing requires a path component only when the package name must be inferred from that path.
An explicit name/version fragment is sufficient without a path component.
The parser still validates the name, version, protocol, and query under existing rules.
No source URL is rewritten, normalized to another transport, or given a fabricated path.

Build Cargo from the same upstream revision with the reviewed patch.
Keep the Rust compiler, rustfmt, Clippy, target libraries, and producer revisions unchanged.
Use the repaired Cargo in the repository Nix shell and Nix command consumers.
Retain an explicit patched-tool identity and the upstream source identity.
Let Nix generate the added source-input lock entry.

## Alternatives and risks

A formatter-only repair leaves an invalid explicit-identity round-trip.
A URL workaround changes identity or transport and previously failed the tested Radicle fetch route.
A newer toolchain introduces unrelated changes. A metadata postprocessor conceals the producer failure.
A sibling checkout is not a durable product dependency.

Risk: accepting malformed fragments. Positive and negative schema tests cover names, versions, bare URLs, empty fragments, queries, and Git references.
Risk: Nix selects the original Cargo through another path. Command and package checks verify the effective executable and compiler cohort.
Risk: a source-only patch passes while real consumers remain broken. Actual metadata and nextest are required.

## Verification and execution limits

Run the existing schema suite before the patch. Add failing round-trip cases before the production correction.
Run the same suite after the patch, then compile and exercise real Cargo.
Run actual Molten metadata with locked dependencies and all features, followed by nextest under the repository profile.
Run applicable formatting, Nix, and Cairn checks. Preserve broader unresolved acceptance gates.

Use eight-minute check deadlines and two build jobs. Record any justified new execution round rather than extending a running command.
The first corrected Nix recipe reached its eight-minute deadline during a cold dependency build.
One new package-build round has a 24-minute limit. This allocation does not change test profiles, runtime timeouts, source behavior, or build parallelism.
Metadata, nextest, and other check commands retain their eight-minute limits.
A command-local compiler-wrapper bypass can isolate schema tests from the observed cache lock.
It does not establish acceptance of the normal cached build path.
Independent review remains advisory and bounded. No review availability claim replaces executable evidence.
