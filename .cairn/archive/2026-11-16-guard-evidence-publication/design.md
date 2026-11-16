## Context

At `fcbf60a3d`, 1,068 tracked files under the scanned roots match the patterns (636 in `evidence/`, 425 in `.cairn/archive/`, 4 in `.cairn/changes/`, 3 in `docs/`), almost all home paths, `aspen1.local`, `/dev/shm` run paths and the tailnet address `100.100.103.95`. Many are bound by receipts, manifests or Nix checks, so the #52 decision preserves them. The guard must stop recurrence without touching them.

## Decisions

### Decision: Scan new and modified evidence; grandfather history by content identity

**Choice:** `scripts/evidence-publication-guard.py` (stdlib Python plus `b3sum`) scans evidence-bearing roots. A file with findings passes only when its path and the BLAKE3 of its bytes match a baseline row. `--write-baseline` bootstraps from `git ls-files` once; afterwards it keeps only unchanged rows or identical bytes at a new path, and refuses everything else. `--all` also fails on stale rows.

**Rationale:** Binding bytes rather than paths makes any edit drop the exemption, so touching history forces redaction. BLAKE3 follows the stack's identity convention; Python's standard library lacks it, so the scanner calls `b3sum`, as `scripts/vendored-sources.py` already does, and fails closed when it is missing. Allowing same-bytes moves keeps Cairn archiving (which moves change evidence) possible without admitting new material.

### Decision: Rules and scope

**Choice:** Under `evidence/`, `release/` and change/archive evidence: home paths (`/home/<user>`, `/Users/<user>`), `/dev/shm/`, `.local` host names, `britton-desktop`, RFC 1918 and 100.64.0.0/10 IPv4 (four bounded octets), and compressed containers. Under `docs/`: home and `/dev/shm/` paths only. The GitHub-hosted runner home `/home/runner` is public and excluded. `/tmp` is not flagged: it is conventional and not identifying.

**Rationale:** These are the classes observed in the inventory. Documentation legitimately names example addresses and the observed host name when describing this issue, so docs are checked only for path leaks. Containers cannot be scanned, so they need an explicit, digest-bound review.

### Decision: Reviewed exemptions

**Choice:** `tools/repository-hygiene/evidence-publication-allowlist.tsv` rows bind a path to one exact matched text (or `opaque-container:<blake3>`) with a reason. Under `docs/` only, an `evidence-publication: allow` marker on the line exempts that line.

**Rationale:** Both are visible in review. Inline markers are limited to docs because evidence files are generated output and should be redacted, not annotated.

### Decision: Enforce archive dates in CI from a recorded set

**Choice:** `tools/repository-hygiene/epoch-archives.txt` records the 88 historical `1970-01-01-*` archives; the `epoch-archive-dates` flake check fails on any other epoch-dated archive and on a missing recorded one. The existing staged-addition pre-commit hook is kept.

**Rationale:** The hook only runs locally; the flake check makes the rule hold in `nix flake check`. Failing on a missing name also guards the preservation rule.

### Decision: Cairn's fallback stays out of scope

**Choice:** Cairn's `archive_date()` epoch fallback lives in the sibling Cairn repository and is not changed here.

**Rationale:** This repository can only reject its effect; the defect belongs to Cairn.

## Risks / Trade-offs

- Patterns can miss host material in unusual forms (other host names, IPv6, user names outside paths); the guard reduces, not eliminates, disclosure, and reviewers still read evidence.
- Editing a grandfathered document (three docs are in the baseline) now requires redacting it in the same change.
- The pre-commit hook needs `b3sum` only for grandfathered or container files; the development shell now provides it.
