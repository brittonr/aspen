#!/usr/bin/env python3
"""Reject host-specific material in newly published evidence (issue #52).

Scanned roots, relative to the repository root:

- ``evidence/``, ``release/``, ``.cairn/changes/<change>/evidence/`` and
  ``.cairn/archive/<archive>/evidence/``: home paths, ``/dev/shm/`` run paths,
  ``.local`` and observed host names, RFC 1918 and CGNAT/tailnet
  (100.64.0.0/10) IPv4 addresses, and opaque compressed containers whose
  members cannot be scanned. The GitHub-hosted runner home (``/home/runner``,
  ``/Users/runner``) is public and not a finding.

Every other path is out of scope. Findings are exempt only through:

- the baseline, which grandfathers a file published before this guard by
  binding its path to the BLAKE3 of its exact bytes, so any edit drops the
  exemption;
- the allowlist, whose reviewed rows exempt one exact matched text (or one
  opaque container by BLAKE3) in one file and record the reason;
- under ``docs/`` only, an ``evidence-publication: allow`` marker on the same
  line as a documentation example.

Usage:
    evidence-publication-guard.py [--root DIR] PATH...   # pre-commit: staged files
    evidence-publication-guard.py [--root DIR] --all     # whole tree, plus stale-baseline check
    evidence-publication-guard.py --write-baseline       # regenerate from `git ls-files`

Exit status: 0 clean, 1 findings, 2 configuration or input error (fail closed).
"""
# r[impl molten.project.evidence_publication.host_material]
import argparse
import bisect
import ipaddress
import os
import re
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

BASELINE = "tools/repository-hygiene/evidence-publication-baseline.tsv"
ALLOWLIST = "tools/repository-hygiene/evidence-publication-allowlist.tsv"
INLINE_MARKER = b"evidence-publication: allow"
MAX_REPORTED_PER_FILE = 20

# Report roots, in the order counts are printed.
ROOTS = ("evidence", ".cairn/changes", ".cairn/archive", "release", "docs")

FULL = frozenset({"home", "scratch", "local-host", "observed-host", "private-ipv4", "tailnet-ipv4", "opaque-container"})
PATHS_ONLY = frozenset({"home", "scratch", "opaque-container"})

PLACEHOLDERS = {
    "home": "<home> (or <workspace> for the checkout)",
    "scratch": "<scratch>",
    "local-host": "<host>",
    "observed-host": "<host>",
    "private-ipv4": "<private-ip>",
    "tailnet-ipv4": "<private-ip>",
    "opaque-container": "publish reviewed members, or allowlist the container by BLAKE3",
}

TEXT_RULES = (
    # GitHub-hosted runners' public home (`/home/runner`, `/Users/runner`) identifies no person or host.
    ("home", re.compile(rb"/(?:home|Users)/(?!runner(?![A-Za-z0-9._-]))[A-Za-z0-9._-]+")),
    ("scratch", re.compile(rb"/dev/shm/")),
    (
        "local-host",
        re.compile(
            rb"(?<![A-Za-z0-9.-])[A-Za-z0-9](?:[A-Za-z0-9-]*[A-Za-z0-9])?"
            rb"(?:\.[A-Za-z0-9](?:[A-Za-z0-9-]*[A-Za-z0-9])?)*\.local(?![A-Za-z0-9_-]|\.[A-Za-z0-9])"
        ),
    ),
    ("observed-host", re.compile(rb"britton-desktop", re.IGNORECASE)),
)
IPV4 = re.compile(rb"(?<![0-9A-Za-z.])(?:[0-9]{1,3}\.){3}[0-9]{1,3}(?![0-9A-Za-z]|\.[0-9])")
PRIVATE_NETWORKS = tuple(
    ipaddress.ip_network(network) for network in ("10.0.0.0/8", "172.16.0.0/12", "192.168.0.0/16")
)
TAILNET_NETWORK = ipaddress.ip_network("100.64.0.0/10")
CONTAINER_MAGIC = (
    b"PK\x03\x04",
    b"PK\x05\x06",
    b"\x1f\x8b",
    b"\x28\xb5\x2f\xfd",
    b"\xfd7zXZ\x00",
    b"7z\xbc\xaf\x27\x1c",
)
BZIP2 = re.compile(rb"BZh[1-9]1AY&SY")


class GuardError(Exception):
    """Configuration or input problem; the guard fails closed."""


@dataclass(frozen=True)
class Finding:
    path: str
    line: int
    column: int
    rule: str
    text: str

    def render(self) -> str:
        return f"{self.path}:{self.line}:{self.column}: {self.rule}: {self.text!r} -> use {PLACEHOLDERS[self.rule]}"


def scope(path: str) -> frozenset | None:
    parts = path.split("/")
    if parts[0] in ("evidence", "release") and len(parts) > 1:
        return FULL
    if parts[0] == ".cairn" and len(parts) > 4 and parts[1] in ("changes", "archive") and parts[3] == "evidence":
        return FULL
    if parts[0] == "docs" and len(parts) > 1:
        return PATHS_ONLY
    return None


def report_root(path: str) -> str:
    for root in ROOTS:
        if path == root or path.startswith(root + "/"):
            return root
    raise GuardError(f"{path}: not under a scanned root")


def is_container(data: bytes) -> bool:
    return data.startswith(CONTAINER_MAGIC) or BZIP2.match(data) is not None


def classify_ipv4(text: bytes) -> str | None:
    try:
        address = ipaddress.ip_address(text.decode("ascii"))
    except ValueError:
        return None
    if address in TAILNET_NETWORK:
        return "tailnet-ipv4"
    if any(address in network for network in PRIVATE_NETWORKS):
        return "private-ipv4"
    return None


def scan_bytes(path: str, data: bytes, rules: frozenset) -> list[Finding]:
    """Return every finding in ``data`` for ``rules``, ignoring exemptions."""
    if is_container(data):
        return [Finding(path, 1, 1, "opaque-container", "compressed container")] if "opaque-container" in rules else []
    raw: list[tuple[int, str, bytes]] = []
    for rule, pattern in TEXT_RULES:
        if rule in rules:
            raw.extend((match.start(), rule, match.group()) for match in pattern.finditer(data))
    if rules & {"private-ipv4", "tailnet-ipv4"}:
        for match in IPV4.finditer(data):
            rule = classify_ipv4(match.group())
            if rule is not None and rule in rules:
                raw.append((match.start(), rule, match.group()))
    if not raw:
        return []
    newlines = [index for index, byte in enumerate(data) if byte == 0x0A]
    findings = []
    for offset, rule, text in sorted(raw):
        line = bisect.bisect_left(newlines, offset)
        line_start = newlines[line - 1] + 1 if line else 0
        findings.append(Finding(path, line + 1, offset - line_start + 1, rule, text.decode("utf-8", "replace")))
    return findings


def line_bytes(data: bytes, line: int) -> bytes:
    lines = data.split(b"\n")
    return lines[line - 1] if 0 < line <= len(lines) else b""


def read_table(path: Path, columns: int) -> list[list[str]]:
    if not path.is_file():
        raise GuardError(f"{path}: missing")
    rows = []
    for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        if not line or line.startswith("#"):
            continue
        fields = line.split("\t")
        if len(fields) != columns or not all(fields):
            raise GuardError(f"{path}:{number}: expected {columns} non-empty tab-separated fields")
        rows.append(fields)
    return rows


def load_baseline(path: Path) -> dict[str, str]:
    rows = read_table(path, 2)
    paths = [row[1] for row in rows]
    if paths != sorted(set(paths)):
        raise GuardError(f"{path}: rows must be unique and sorted by path")
    baseline = {}
    for digest, entry in rows:
        if not re.fullmatch(r"[0-9a-f]{64}", digest):
            raise GuardError(f"{path}: {entry}: malformed BLAKE3 digest")
        if scope(entry) is None:
            raise GuardError(f"{path}: {entry}: not under a scanned root")
        baseline[entry] = digest
    return baseline


def load_allowlist(path: Path) -> set[tuple[str, str]]:
    allowed = set()
    for entry, text, _reason in read_table(path, 3):
        if scope(entry) is None:
            raise GuardError(f"{path}: {entry}: not under a scanned root")
        allowed.add((entry, text))
    return allowed


class Hasher:
    def __init__(self, b3sum: str | None):
        self.b3sum = b3sum

    def digest(self, path: Path) -> str:
        if self.b3sum is None:
            raise GuardError("b3sum is required to check BLAKE3 bindings; install b3sum or pass --b3sum")
        output = subprocess.run(
            [self.b3sum, "--no-names", "--", str(path)], check=True, stdout=subprocess.PIPE, text=True
        ).stdout
        return output.strip()


class Guard:
    def __init__(self, root: Path, baseline: dict[str, str], allowlist: set[tuple[str, str]], hasher: Hasher):
        self.root = root
        self.baseline = baseline
        self.allowlist = allowlist
        self.hasher = hasher

    def raw_findings(self, path: str) -> tuple[bytes, list[Finding]]:
        rules = scope(path)
        if rules is None:
            return b"", []
        file = self.root / path
        if file.is_symlink() or not file.is_file():
            raise GuardError(f"{path}: not a regular file")
        data = file.read_bytes()
        return data, scan_bytes(path, data, rules)

    def unexempted(self, path: str, data: bytes, findings: list[Finding]) -> list[Finding]:
        remaining = []
        for finding in findings:
            if finding.rule == "opaque-container":
                token = f"opaque-container:{self.hasher.digest(self.root / path)}"
            else:
                token = finding.text
            if (path, token) in self.allowlist:
                continue
            if path.startswith("docs/") and INLINE_MARKER in line_bytes(data, finding.line):
                continue
            remaining.append(finding)
        return remaining

    def violations(self, path: str) -> tuple[list[Finding], bool]:
        """Return (violations, grandfathered) for one repository-relative path."""
        data, findings = self.raw_findings(path)
        remaining = self.unexempted(path, data, findings)
        if not remaining:
            return [], False
        expected = self.baseline.get(path)
        if expected is not None and self.hasher.digest(self.root / path) == expected:
            return [], True
        return remaining, False


def relative(root: Path, argument: str) -> str:
    path = Path(argument)
    absolute = path if path.is_absolute() else Path.cwd() / path
    try:
        rel = absolute.resolve().relative_to(root.resolve())
    except ValueError as error:
        raise GuardError(f"{argument}: outside the repository root {root}") from error
    return rel.as_posix()


def is_git_checkout(root: Path) -> bool:
    return (root / ".git").exists()


def git_files(root: Path, include_untracked: bool) -> list[str]:
    command = ["git", "-C", str(root), "ls-files", "-z", "--cached"]
    if include_untracked:
        command += ["--others", "--exclude-standard"]
    output = subprocess.run(command, check=True, stdout=subprocess.PIPE).stdout
    return sorted({entry.decode("utf-8") for entry in output.split(b"\0") if entry})


def walked_files(root: Path) -> list[str]:
    found = []
    for top in ("evidence", ".cairn", "release", "docs"):
        base = root / top
        if not base.is_dir():
            continue
        for directory, _dirs, files in os.walk(base):
            for name in files:
                found.append((Path(directory) / name).relative_to(root).as_posix())
    return sorted(found)


def candidate_files(root: Path, include_untracked: bool) -> list[str]:
    files = git_files(root, include_untracked) if is_git_checkout(root) else walked_files(root)
    return [path for path in files if scope(path) is not None and (root / path).is_file()]


def print_findings(findings: list[Finding]) -> None:
    for finding in findings[:MAX_REPORTED_PER_FILE]:
        print(finding.render(), file=sys.stderr)
    if len(findings) > MAX_REPORTED_PER_FILE:
        print(f"{findings[0].path}: ... {len(findings) - MAX_REPORTED_PER_FILE} more findings", file=sys.stderr)


def check(guard: Guard, paths: list[str], whole_tree: bool) -> int:
    failed = 0
    grandfathered = set()
    scanned = 0
    for path in paths:
        if scope(path) is None:
            continue
        scanned += 1
        findings, exempt = guard.violations(path)
        if exempt:
            grandfathered.add(path)
        if findings:
            failed += 1
            print_findings(findings)
    stale = []
    if whole_tree:
        present = set(paths)
        for path in sorted(guard.baseline):
            if path not in present:
                stale.append(f"{path}: stale baseline entry; the file is gone")
            elif path not in grandfathered and not guard.violations(path)[0]:
                stale.append(f"{path}: stale baseline entry; the file no longer needs an exemption")
    for message in stale:
        print(message, file=sys.stderr)
    print(
        f"evidence-publication: scanned {scanned} files, {len(grandfathered)} grandfathered, "
        f"{failed} rejected, {len(stale)} stale baseline entries"
    )
    if failed:
        print(
            "Redact with the placeholders in CONTRIBUTING.md (Publishing evidence); "
            "do not add new files to the baseline.",
            file=sys.stderr,
        )
    return 1 if failed or stale else 0


def write_baseline(root: Path, baseline_path: Path, guard: Guard) -> int:
    """Regenerate the baseline from tracked files.

    Without a previous baseline every current finding is grandfathered (the one-time bootstrap).
    Afterwards a row survives only for unchanged bytes at its path, or for the same bytes at a new
    path (an archived change moves its evidence); any other finding is refused, not grandfathered.
    """
    if not is_git_checkout(root):
        raise GuardError("--write-baseline needs a git checkout: the baseline is generated from `git ls-files`")
    bootstrap = not baseline_path.is_file()
    previous = {} if bootstrap else load_baseline(baseline_path)
    previous_digests = set(previous.values())
    rows = []
    refused = []
    for path in candidate_files(root, include_untracked=False):
        data, findings = guard.raw_findings(path)
        if not guard.unexempted(path, data, findings):
            continue
        if "\t" in path or "\n" in path:
            raise GuardError(f"{path!r}: path cannot be recorded in a TSV baseline")
        digest = guard.hasher.digest(root / path)
        if bootstrap or previous.get(path) == digest or (path not in previous and digest in previous_digests):
            rows.append((path, digest))
        else:
            refused.append(path)
    lines = [
        "# Grandfathered evidence-publication findings (issue #52); generated by",
        "# `python3 scripts/evidence-publication-guard.py --write-baseline` from `git ls-files`.",
        "# Each row binds a path to the BLAKE3 of its exact bytes. Editing the file drops",
        "# its exemption. Rows may only disappear or move with identical bytes.",
        "# blake3\tpath",
    ]
    lines += [f"{digest}\t{path}" for path, digest in rows]
    baseline_path.parent.mkdir(parents=True, exist_ok=True)
    baseline_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    counts = {root_name: 0 for root_name in ROOTS}
    for path, _digest in rows:
        counts[report_root(path)] += 1
    current = {path for path, _digest in rows}
    print(f"evidence-publication baseline: {len(rows)} files")
    for root_name in ROOTS:
        print(f"  {root_name}: {counts[root_name]}")
    print(f"  new paths {len(current - set(previous))}, dropped {len(set(previous) - current)}")
    for path in refused:
        print(f"{path}: not grandfathered; redact it", file=sys.stderr)
    return 1 if refused else 0


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("paths", nargs="*", help="files to check (pre-commit passes staged files)")
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--all", action="store_true", help="check every file under the scanned roots")
    mode.add_argument("--write-baseline", action="store_true", help="regenerate the baseline from git ls-files")
    parser.add_argument("--root", default=".", help="repository root (default: current directory)")
    parser.add_argument("--b3sum", default=shutil.which("b3sum"), help="b3sum executable")
    args = parser.parse_args(argv)
    root = Path(args.root)
    try:
        if (args.all or args.write_baseline) and args.paths:
            raise GuardError("--all and --write-baseline take no paths")
        if not (args.all or args.write_baseline or args.paths):
            raise GuardError("no paths given; pass files, --all or --write-baseline")
        hasher = Hasher(args.b3sum)
        allowlist = load_allowlist(root / ALLOWLIST)
        if args.write_baseline:
            return write_baseline(root, root / BASELINE, Guard(root, {}, allowlist, hasher))
        guard = Guard(root, load_baseline(root / BASELINE), allowlist, hasher)
        if args.all:
            return check(guard, candidate_files(root, include_untracked=True), whole_tree=True)
        return check(guard, sorted({relative(root, path) for path in args.paths}), whole_tree=False)
    except (GuardError, OSError, subprocess.CalledProcessError) as error:
        print(f"evidence-publication: error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
