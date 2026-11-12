# Contributing to Molten

Read the [contributor handbook](docs/handbook/README.md) and the repository
[agent and workflow notes](AGENTS.md) before changing behavior. The
[documentation map](docs/README.md) distinguishes governing documents from the
source-linked handbook and technical companion.

## Workflow

- The mainline is `molten`; the remote is `git@github.com:brittonr/aspen.git`.
  Do not merge or overwrite the legacy `main` branch, force-push, or open a pull
  request. File an issue first, then prepare a repository-local change.
- Significant behavior, contract, or evidence changes use the Cairn lifecycle
  under `.cairn/`: proposal, design, tasks, implementation, validation, review,
  sync, and archive. Use `cairn change list --root .` for the active set.
- Keep commits repository-local and preserve unrelated work. State the exact
  checks you actually ran; do not present a plan, a fixture, or an old receipt as
  a new execution result.
- Small documentation or maintenance edits may go directly to `molten` when the
  owning guides and repository notes allow it.

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
author's work as an original Molten contribution. Licensing metadata is
distribution evidence, not legal advice.

## Security and conduct

Use [SECURITY.md](SECURITY.md) for suspected vulnerabilities instead of a public
issue. A code of conduct is a separate community choice and is not implied by
this contribution guide.
