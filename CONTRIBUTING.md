# Contributing to Aspen

<a id="contributing-to-molten"></a>

Start with the [README](README.md) for Aspen's purpose, current status, and first
local workflow. The [contributor handbook](docs/handbook/README.md) covers setup,
focused checks, and preparing a change. Read the repository's
[workflow notes](AGENTS.md) before changing behavior; the
[documentation map](docs/README.md) points to the governing subsystem contracts.

Use **Aspen** as the project name in new documentation. **Molten** is an
implementation codename still used in branch, executable, crate, and protocol
identifiers. Keep those literal names intact unless the change explicitly
addresses compatibility.

Aspen is the public project name. Molten is its internal codename; the current
`molten` branch, crates, executable and canonical schema names remain unchanged.

## Workflow

- The mainline is `molten`; the remote is `git@github.com:brittonr/aspen.git`.
  Do not merge or overwrite the legacy `main` branch, force-push, or open a pull
  request. File an issue first, then prepare a repository-local change.
- Significant behavior, contract, or evidence changes use the Cairn lifecycle
  under `.cairn/`: proposal, design, tasks, implementation, validation, review,
  sync, and archive. Use `cairn change list --root .` for the active set. Archive
  with an explicit date (`CAIRN_ARCHIVE_DATE=$(date -u +%F)`); without it Cairn
  names the archive `1970-01-01-*`, which the `cairn-archive-date` pre-commit
  hook rejects. The `epoch-archive-dates` flake check rejects any
  `1970-01-01-*` archive outside the historical set recorded in
  `tools/repository-hygiene/epoch-archives.txt`.
- Keep commits repository-local and preserve unrelated work. State the exact
  checks you actually ran; do not present a plan, a fixture, or an old receipt as
  a new execution result.
- Small documentation or maintenance edits may go directly to `molten` when the
  owning guides and repository notes allow it.

## Publishing evidence

Files you add or change under `evidence/`, `release/`,
`.cairn/changes/<change>/evidence/` or `.cairn/archive/<archive>/evidence/` must
not carry host-specific material. Redact before you compute digests or write a
receipt that binds the bytes:

| Material | Placeholder |
| --- | --- |
| The checkout or worktree under `/home/<user>/` or `/Users/<user>/` | `<workspace>` |
| Any other home path | `<home>` |
| `/dev/shm/<run>/…` scratch paths | `<scratch>/<run>/…` |
| `*.local` and other machine host names | `<host>` |
| RFC 1918 and CGNAT/tailnet IPv4 (`10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`, `100.64.0.0/10`) | `<private-ip>` |

Under `docs/`, only home and `/dev/shm/` paths are checked. Keep an unredacted
original privately when the redacted copy cannot reproduce an identity you need
(see [SECURITY.md](SECURITY.md)).

The `evidence-publication-guard` pre-commit hook checks staged files, and the
`evidence-publication` flake check runs the scanner's fixture tests and
`python3 scripts/evidence-publication-guard.py --all`. Findings are reported as
`path:line:column: rule` with the placeholder to use. Compressed containers
cannot be scanned and are rejected.

Files published before the guard are grandfathered by
`tools/repository-hygiene/evidence-publication-baseline.tsv`, which binds each
path to the BLAKE3 of its exact bytes. Editing such a file drops its exemption,
so redact it in the same change. Never add new files to the baseline:
`--write-baseline` regenerates it from `git ls-files` and keeps a row only for
unchanged bytes at its path or the same bytes at a new path (archiving a change
moves its evidence; run it after the move is staged). It refuses every other
finding. The `--all` check rejects rows whose file is gone or no longer needs an
exemption. The GitHub-hosted runner home (`/home/runner`, `/Users/runner`) is
public and is not a finding.

Genuine, non-identifying examples need a reviewed exemption: under `docs/`, put
`evidence-publication: allow` on the example's line (for instance in an HTML
comment); anywhere in scope, add a row with the path, the exact matched text and
the reason to `tools/repository-hygiene/evidence-publication-allowlist.tsv`. A
container whose members you have reviewed is allowlisted as
`opaque-container:<blake3>`.

## Build and focused checks

Run commands from the repository inside the Nix development shell:

```sh
nix develop
cargo check --workspace --all-targets
cargo test -p molten --lib <focused-filter>
cargo nextest run --profile fast-core
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
```

The default public source graph uses immutable HTTPS pins for the OnixResearch
GitHub dependencies. A fresh checkout does not need a GitHub SSH key for those
sources. Private Radicle inputs remain explicit and require their documented
public seed or a local Radicle profile. Sibling checkouts are development-only
`--override-input` substitutions and are not release evidence.

For source-gate changes, follow the ordered Octet sequence in the
[README](README.md#development). For lifecycle changes, run strict Cairn
validation before sync or archive. Use the repository's Nix checks for the
surface you touched rather than assuming an ambient toolchain is equivalent.

## Contribution license

Submit original contributions under `AGPL-3.0-or-later`, the project license in
[LICENSE](LICENSE). You retain your copyright. Submit only material you have the
right to contribute, including any required employer permission.

Earlier permissive grants remain effective for byte-identical material already
received under them. For third-party or vendored material, preserve the original
license and attribution, identify the immutable upstream revision and local
modifications, include the required texts, and update
[THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md). Do not present an upstream
author's work as an original Aspen contribution. Licensing metadata is
distribution evidence, not legal advice.

## Security and conduct

Use [SECURITY.md](SECURITY.md) for suspected vulnerabilities instead of a public
issue. A code of conduct is a separate community choice and is not implied by
this contribution guide.
