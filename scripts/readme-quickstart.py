#!/usr/bin/env python3
"""Run the README "Getting started" fixture commands (issues #25 and #29).

The packaged-binary mode substitutes --molten for `cargo run --locked --` and
executes in a fresh temporary working directory. The --cargo mode executes the
unchanged Cargo commands from the README's repository root; the documented
mktemp command isolates its output, and the documented cleanup removes it.

Both modes require successful commands, the promised fixture/stopped status
results, a retained status artifact before cleanup, and no run directory after
cleanup. Neither executes the clone / `nix develop` block. The hosted
reproducible-dependencies workflow owns anonymous bootstrap, the locked Cargo
build and default Nix build before its verbatim --cargo quickstart.

A packaged-binary pass is not a verbatim Cargo or cold-source observation.

Usage:
    readme-quickstart.py (--molten BIN | --cargo) [--readme README.md]

Exit status: 0 pass, 1 failure, 2 input error.
"""
# r[impl molten.project.documentation_links.quickstart_commands]
import argparse
from contextlib import nullcontext
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

SECTION = "## Getting started"
CARGO_INVOCATION = "cargo run --locked --"
CLEANUP_MARKER = 'rm -rf -- "${ASPEN_DEMO_DIR'
DEMO_VARIABLE = "ASPEN_DEMO_DIR"
EXPECTED_RESULTS = (
    "system-extension fixture ok profile=sandboxed-component",
    "phase=stopped",
    "health=stopped",
)
FENCE = re.compile(r"^```(\w*)\s*$")
TIMEOUT_SECONDS = 600


def section_blocks(readme: str) -> list[str]:
    """Fenced shell blocks between the Getting started heading and the next level-2 heading."""
    _, found, rest = readme.partition(f"\n{SECTION}\n")
    if not found:
        raise ValueError(f"README has no '{SECTION}' section")
    section = re.split(r"\n## ", rest, maxsplit=1)[0]
    blocks, current = [], None
    for line in section.splitlines():
        fence = FENCE.match(line)
        if current is None and fence and fence.group(1) in ("sh", "bash", "shell"):
            current = []
        elif current is not None and fence:
            blocks.append("\n".join(current))
            current = None
        elif current is not None:
            current.append(line)
    return blocks


def quickstart_script(readme: str, *, substitute_binary: bool) -> tuple[str, str]:
    """Return the run and cleanup blocks, optionally selecting the packaged binary."""
    blocks = section_blocks(readme)
    runs = [block for block in blocks if CARGO_INVOCATION in block]
    cleanups = [block for block in blocks if CLEANUP_MARKER in block]
    if len(runs) != 1 or len(cleanups) != 1:
        raise ValueError(f"expected one run block and one cleanup block, found {len(runs)} and {len(cleanups)}")
    run_block = runs[0].replace(CARGO_INVOCATION, '"$MOLTEN_BIN"') if substitute_binary else runs[0]
    return run_block, cleanups[0]


def run(readme_path: Path, molten: Path | None) -> list[str]:
    readme = readme_path.read_text(encoding="utf-8")
    failures = [f"README no longer states `{result}`" for result in EXPECTED_RESULTS if f"`{result}" not in readme]
    run_block, cleanup_block = quickstart_script(readme, substitute_binary=molten is not None)
    workspace = (
        tempfile.TemporaryDirectory(prefix="readme-quickstart-")
        if molten is not None
        else nullcontext(readme_path.parent)
    )
    with workspace as workdir:
        script = (
            "set -euo pipefail\n"
            f"{run_block}\n"
            f'test -f "${DEMO_VARIABLE}/status.preserves"\n'
            f"{cleanup_block}\n"
            f'test ! -e "${DEMO_VARIABLE}"\n'
            "test -d target\n"
        )
        environment = dict(os.environ)
        if molten is not None:
            environment["MOLTEN_BIN"] = str(molten)
        completed = subprocess.run(
            ["bash", "-c", script],
            cwd=workdir,
            env=environment,
            capture_output=True,
            text=True,
            timeout=TIMEOUT_SECONDS,
        )
        output = completed.stdout + completed.stderr
        if completed.returncode != 0:
            failures.append(f"quick-start commands exited {completed.returncode}:\n{output}")
        for result in EXPECTED_RESULTS:
            if result not in completed.stdout:
                failures.append(f"output lacks `{result}`")
        print(completed.stdout, end="")
    return failures


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--molten", type=Path)
    mode.add_argument("--cargo", action="store_true")
    parser.add_argument("--readme", type=Path, default=Path(__file__).resolve().parent.parent / "README.md")
    arguments = parser.parse_args(argv)
    if not arguments.readme.is_file() or (arguments.molten is not None and not os.access(arguments.molten, os.X_OK)):
        print("README or executable missing", file=sys.stderr)
        return 2
    try:
        molten = arguments.molten.resolve() if arguments.molten is not None else None
        failures = run(arguments.readme.resolve(), molten)
    except ValueError as error:
        print(f"README quick start: {error}", file=sys.stderr)
        return 2
    for failure in failures:
        print(f"README quick start: {failure}", file=sys.stderr)
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
