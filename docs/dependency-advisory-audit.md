# Aspen resolved dependency advisory and license audit

The default release-profile observation uses maintained **cargo-audit 0.23.0**
(released 2026-11-10), checksum-pinned in the dedicated
[Nix module](../nix/dependency-audit.nix). Its published Cargo.lock pins scanner
build dependencies. The old caller-authored exact-version/star advisory input
is removed: an empty supplied list is not a maintained RustSec scan.

The normalizer is built from the same Rust source with a mechanically selected
subset of the existing workspace lock cohort plus the checksum-pinned SPDX
parser. The Nix app uses that immutable binary, not a freshly resolved cargo
script. No workspace lock or generated build plan is changed for this tool.

This adds release-review observations. Existing owned-license/notice,
Git-source hash and release-dependency guards remain prerequisites and their
actual outputs/statuses are retained, not replaced by a second authority.
A pass does not grant release approval, legal/distribution compliance,
runtime security qualification or a vulnerability-free claim. Offline scanning
does not query live registry yanked status. Source-specific advisory coverage
is limited to what the selected RustSec database actually contains.

## Explicit inputs and profile

Only `molten-default` on an explicit target is selected. Locked, offline,
target-filtered Cargo metadata retains resolved package features and paths
from the Aspen root (`molten` package). The scanner conservatively evaluates the entire lockfile
without target/severity suppression. Findings are projected onto the resolved
selected closure. Optional features need a separately reviewed profile; no
all-features baseline is used.

Bindings include actual raw scanner JSON, scanner executable/version, immutable
RustSec commit/tree/date, Cargo.lock, Cargo/rustc identities, source inventory
hashes (manifest, flake lock, build plans, release profile, license policy,
tool and wrapper), and prerequisite guard output/status. Source/build input
identities do not prove that a binary was built or qualified; no compiled-output
evidence is synthesized.

The pinned scanner's `--no-fetch` path leaves its Git commit/date fields null.
The capture independently verifies the clean maintained Git snapshot before and
after scanning. Readback recomputes the retained commit's Git object identity
and derives its UTC date from its committer timestamp; any scanner-supplied
metadata must agree. Missing fields, a changed commit/date or a stale snapshot
still deny. The original scanner JSON is never rewritten to fill those fields.

A clean Git checkout of `https://github.com/RustSec/advisory-db` at an explicit
40-hex commit is required. Fetching/refreshing is separate and creates a new
observation. The runner never fetches advisory or registry data. A database
older than 14 days, future date, empty database, database mismatch, scanner pin
older than 90 days, changed binding, unavailable metadata or failed prerequisite
denies. Private Git/Radicle sources and the existing release evidence may need
credentials/materialization before going offline; unavailable inputs never
become clean evidence.

## Run the offline Nix app

```sh
# Deliberate external refresh: review and select an immutable commit.
git clone https://github.com/RustSec/advisory-db /path/to/advisory-db
git -C /path/to/advisory-db checkout --detach "$REVIEWED_RUSTSEC_COMMIT"

nix run .#dependency-audit -- \
  --database /path/to/advisory-db \
  --database-revision "$REVIEWED_RUSTSEC_COMMIT" \
  --target x86_64-unknown-linux-gnu --as-of 2026-11-13 \
  --out target/dependency-audit-20261113 \
  --release-evidence-source valence-integrity=/path/to/exact-valence \
  --release-evidence-source octet-cutover=/path/to/exact-octet \
  --release-evidence-source hardened-wasmtime-resumable=/path/to/exact-hardened-wasmtime \
  --release-evidence-source mantle-wasm-consumer-verifier=/path/to/exact-mantle
```

Use a new output directory per observation. The bundle retains `scanner.json`,
`scanner.stderr`, metadata, exported policies, guard outputs and stderr,
`status.json`, `binding.json`, complete `report.json`, a compact `summary.md`
and content-addressed notice bytes. Child commands have a 300-second deadline
and 32 MiB per-file ceiling. Failed captures are unavailable, never silently
truncated into success. The human summary stops at 100 findings; machine output
is complete. Summaries/logs are adjuncts, not canonical release authority.

## Sources, SPDX and notices

Registry packages use the canonical crates.io source; Git/Radicle coordinates
and revisions must exactly match the existing release profile. Organization
prefixes and arbitrary seeds do not authorize sources. Unrecognized/path
sources outside workspace packages deny.

The maintained `spdx` parser evaluates real AND/OR/WITH grammar against the
existing audit license choices. Malformed/missing metadata denies; a
license-file path is not a substitute for an SPDX expression. Local package
license/notice/copying/copyright artifacts are measured and copied by hash;
missing/unreadable/out-of-bounds evidence denies. Declared paths escaping a
package root are not notice evidence. This records artifacts, not a legal
finding that every obligation of a distribution has been satisfied.
Owned workspace packages reuse the workspace-root license artifacts and existing
license guard, rather than requiring duplicate copies of the root license.
Notice directories are bounded to four levels and 256 children per directory;
individual artifacts are capped at 1 MiB and total copied bytes at 32 MiB.
Unavailable declared/discovered artifacts remain explicit in the captured
notice inventory even when another license file was successfully captured.

## Reviewed exceptions

No exceptions are shipped. An advisory exception binds exact package, version,
source and advisory ID, owner, rationale, HTTPS review identity, review date
and expiry. Wildcards and package-wide exceptions are prohibited. Lifetime is
at most 90 days; expiry is inclusive. Invalid or expired reviews deny even if
the package/advisory is no longer selected. An exception whose exact
package/version/source is absent from the selected closure is a stale binding
and also denies. Source, unavailable-license and notice findings cannot be
suppressed by advisory exceptions. Excepted findings retain the full review
plus original scanner evidence; they are not clean.

## Controls and reproducibility

Executable controls cover affected packages, unknown sources, unavailable or
invalid licenses, expired exceptions, stale scanner/database and changed
bindings. The time 0.1.43 lock fixture exercises actual maintained RustSec
RUSTSEC-2020-0071 matching. Pure normalizer fixtures are labeled controls, not
an actual workspace scan.

Run the deterministic normalizer controls and the real maintained-database affected
fixture (requires the same explicit clean RustSec checkout):

```sh
nix run .#dependency-audit-controls -- \
  --database /path/to/advisory-db \
  --database-revision "$REVIEWED_RUSTSEC_COMMIT" \
  --out target/dependency-audit-controls-20261113
nix build .#checks.x86_64-linux.dependency-audit-scanner-fixture
```

The hermetic Nix scanner fixture uses a clearly synthetic database and does not
claim to scan maintained advisory data or the workspace. The controls app uses
the real supplied RustSec snapshot for the time fixture. Re-normalization uses
the same immutable captured inputs and as-of date; source/executable/capture
changes deny instead of quietly rebinding. Refreshing scanner/database inputs
requires a new observation. Adding the implementation is not evidence that
any audit was executed: real execution needs the external inputs above.

## Observed frozen snapshot — 2026-11-13

The real workspace capture completed in **68.238 seconds** using the shipped
Nix app, with its source-equality checks intact, RustSec commit
`4ef2df8e806a65e2f1ecc14f56f51249062b23d6` (UTC date `2026-11-13`), target
`x86_64-unknown-linux-gnu`, default features and as-of date `2026-11-13`.
The result was **deny: 741 resolved packages, 204 findings**, not a clean audit:

| Captured finding class | Count | Meaning |
| --- | ---: | --- |
| Advisory | 67 | 38 vulnerability, 17 unsound and 12 unmaintained observations from the real maintained scanner |
| License policy | 5 | SPDX expressions outside the reviewed audit license choices |
| Source policy | 5 | Source/revision without an exact existing release-profile binding |
| Unavailable license | 40 | Selected-package license metadata could not establish an accepted expression |
| Unavailable notices | 87 | Selected-package notice evidence was missing or otherwise unavailable |

The last two rows are captured package-evidence deficiencies, not evidence that
the scanner or capture prerequisite was unavailable. Metadata, policy export,
release-profile export, owned-license guard, release guard, source-hash guard,
notice capture and Cargo/rustc identity commands all exited **0**. The scanner
exited **1** with findings; the normalizer exited **1** for deny. There were no
`prerequisite-unavailable`, `scanner-unavailable` or `stale-binding` findings in
this observation. Passing prerequisite guards does not erase the audit's
separate source/SPDX/notice findings. No dependency upgrade or exception was
introduced to turn this result green.

The exact app was realized directly from
`/nix/store/g9449275h2119in37c2c6cjn9c9rbgvj-molten-dependency-audit.drv`
as `/nix/store/jy47hh5hcs5a1jlw0cry6vwa0aij34bf-molten-dependency-audit/bin/molten-dependency-audit`.
It selected cargo-audit `0.23.0`, the immutable normalizer and pinned
`nightly-2026-05-26` toolchain. The release guard used the exact four
`--release-evidence-source` mappings recorded in
[`execution.json`](../.cairn/archive/2026-11-15-audit-resolved-release-dependencies/evidence/2026-11-13/execution.json),
which retains the complete capture and normalization commands, exit statuses,
elapsed time and output comparisons. Its raw output reports
`decision=pass rows=33 archives=4`; this is a prerequisite observation, not
release approval.

The fresh output directory was
`/dev/shm/aspen-audit-workspace-complete-20261113`. During a coordinated source
freeze, its before/after inventories matched across **9,941 source files**.
After capture, the pinned normalizer ran **twice more** against that same bundle,
source root and as-of date. Both runs returned the same deny result; complete
`report.json` and `summary.md` bytes matched each other and the capture's initial
normalization, and `binding.json` remained unchanged:

| Retained bytes | BLAKE3 |
| --- | --- |
| `binding.json` | `f15f7d39423651267302313835630e042be48cd27b9cee700501001bddff987d` |
| `report.json` | `cd147370af9c586e7f6e9c08078f15459d62019843f5985c57128d6911b6e7d0` |
| `scanner.json` | `2314e282d93f6b7dcde790be3b590a7f8b44a011e1a7c70f5b83633de63d94ef` |
| Source inventory | `530fb17b83a68d429d4c4f5800d70c1cb1233204d2d9f997f269f226ac052c11` |

The lifecycle evidence retains the
[complete machine report](../.cairn/archive/2026-11-15-audit-resolved-release-dependencies/evidence/2026-11-13/capture/report.json),
[bindings](../.cairn/archive/2026-11-15-audit-resolved-release-dependencies/evidence/2026-11-13/capture/binding.json),
raw scanner output, metadata, policies, notice bytes, source inventories,
database commit/tree, prerequisite output/status/stderr and replay comparisons.
Scanner-home, Cargo/build caches and the RustSec checkout are not copied.
This binds the **observed pre-documentation snapshot**, not later documentation,
evidence, lifecycle or integration edits. Replaying against a changed source tree
must deny stale bindings; it is not permission to rebind this observation.

The unchanged real-database controls had already passed and were not rerun.
Their retained
[`controls.json`](../.cairn/archive/2026-11-15-audit-resolved-release-dependencies/evidence/2026-11-13/controls/controls.json)
and normalizer output identify fixture-only observations, including the real
RustSec affected-package fixture; they are not substituted for the workspace
scan. The earlier `/dev/shm/aspen-audit-workspace-official-20261113` attempt is
retained separately: its 207 findings included a missing release-evidence
prerequisite and source drift. It is not the reproducible result above. The
earlier bound-attempt path was absent at follow-up inspection, so its timeout
provides no completed scan evidence.

This execution closes the actual-scan/reproducibility evidence gap, not the
findings themselves. Final Nix rails and strict lifecycle validation,
review/sync/archive and integration remain separate parent-owned gates; this
observation makes no all-gates, release-readiness or vulnerability-free claim.
