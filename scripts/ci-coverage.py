#!/usr/bin/env python3
"""Fail-closed CI coverage audits for the Aspen workspace.

Subcommands (each exits 1 with one diagnostic per line on stderr when it denies):

  nextest-scope          The `ci` and `deterministic` nextest profiles name every workspace member.
  junit-members          A nextest JUnit report executed (ran, not skipped) at least one test of every
                         listed member.
  ignored-tests          Every `#[ignore]` test is listed in the reviewed inventory with a closed
                         prerequisite category. A same-host test names a rail whose own definition selects
                         it: a Nix rail's flake binding (and the files it imports) holds the selector and
                         verifies its run with `ignored-rail-evidence`; a workflow rail passes the selector to
                         an `--ignored` cargo test command.
  ignored-rail-evidence  A Nix rail's libtest output shows that every inventory test assigned to the rail
                         ran and passed.
  feature-tests          From a union-feature `cargo test --no-run` build, run every test that exists only
                         under the features (per target, against the default-feature test binaries) and
                         require each feature-gated target to execute tests, unless the reviewed
                         classification records why it cannot.
  hegel-coverage         `crates/molten-core` declares at least one `#[hegel::test]` property and, with
                         `--junit`, every declared property executed and passed.
  octet-baseline         Workspace Octet findings match the reviewed baseline of individual finding
                         identities exactly (`check`), or rewrite that baseline from a fresh run (`write`).
                         An identity is crate, file, lint, and an anchor: a hash of the finding's
                         whitespace-normalized line with its two nearest non-blank lines above and below, plus
                         that window's ordinal among identical windows in the file. It survives unrelated line
                         shifts, and a finding repaired on one line cannot hide a new one on an identical line.

r[impl molten.testing.ci_workspace_member_coverage]
r[impl molten.testing.feature_and_ignore_ci_accounting]
r[impl molten.testing.molten_core_hegel_properties]
r[impl molten.evidence_gates.ci_octet_and_cairn_hermetic]
"""
import argparse
import bisect
import hashlib
import json
import os
import re
import subprocess
import sys
import tomllib
import xml.etree.ElementTree as ElementTree
from pathlib import Path

sys.dont_write_bytecode = True

AGGREGATE_PROFILES = ("ci", "deterministic")
RUST_SOURCE_ROOTS = ("src", "tests", "crates")
IGNORE_ATTRIBUTE = re.compile(r"#\[ignore(?P<rest>[^\]]*)\]")
IGNORE_REASON = re.compile(r'^\s*=\s*"(?P<reason>[^"]+)"\s*$')
FUNCTION = re.compile(r"\bfn\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)")
HEGEL_ATTRIBUTE = re.compile(r"#\[hegel::test\b")
HEGEL_SOURCE_ROOT = "crates/molten-core"
HEGEL_PACKAGE = "molten-core"
# Category -> dispositions a reviewer may record for it. Same-host categories must run on a rail.
CATEGORY_DISPOSITIONS = {
    "swtpm-disposable": {"rail"},
    "pinned-host-tools": {"rail"},
    "nix-built-cohort": {"rail"},
    "local-s3-endpoint": {"rail"},
    "signed-source-trust": {"environment-blocked"},
    "nvme-device": {"environment-blocked"},
    "ebpf-replay-capture": {"environment-blocked"},
    "owner-approved-evidence": {"environment-blocked"},
    "external-artifact-cohort": {"environment-blocked"},
    "diagnostic-only": {"reviewed-exclusion"},
    "role-entrypoint": {"reviewed-exclusion"},
}
RAIL_KINDS = ("nix:", "workflow:")
NIX_IMPORTED_FILE = re.compile(r"(?<![\w.])\./(?P<path>[\w./-]+\.nix)\b")
NIX_IDENTIFIER = re.compile(r"^[A-Za-z_][A-Za-z0-9_'-]*$")
WORKFLOW_TEST_COMMAND = re.compile(r"\bcargo\s+(?:test|nextest\s+run)\b")
LIBTEST_CASE = re.compile(r"^test (?P<name>\S+) \.\.\. ?(?P<rest>.*)$")
LIBTEST_RESULT = re.compile(
    r"^test result: (?P<status>ok|FAILED)\. (?P<passed>\d+) passed; (?P<failed>\d+) failed; (?P<ignored>\d+) ignored",
    re.MULTILINE,
)
# A crate-level `#![cfg(...)]` that requires a Cargo feature makes the whole test target feature-gated.
FEATURE_CRATE_CFG = re.compile(r"^\s*#!\[cfg\((?!not\()(?P<predicate>[^\]]*\bfeature\s*=[^\]]*)\)\]", re.MULTILINE)
FEATURE_DISPOSITIONS = ("ignored-inventory", "environment-blocked")
OCTET_INDEX_LINE = re.compile(
    r"^\s*F\d+\s+(?P<lint>[a-z_]+)\s+(?P<crate>[A-Za-z0-9_-]+)\s+(?P<file>\S+?):(?P<line>\d+)\s*$"
)
# Members whose Octet debt is pinned at zero: every package in `[workspace.metadata.octet] default_scope`,
# which the strict source gate checks with plain `cargo octet check`. Any finding in them fails regardless
# of the baseline. Octet names library targets with underscores, so both spellings are pinned.
def octet_zero_crates(root: Path) -> set[str]:
    manifest = tomllib.loads((root / "Cargo.toml").read_text())
    scope = manifest.get("workspace", {}).get("metadata", {}).get("octet", {}).get("default_scope", [])
    packages = {value for flag, value in zip(scope, scope[1:]) if flag in ("-p", "--package")}
    return packages | {package.replace("-", "_") for package in packages}


BASELINE_SCHEMA = "molten-octet-workspace-baseline-v3"
ANCHOR_CONTEXT = 2
ANCHOR_DESCRIPTION = (
    "sha256, first 16 hex digits, of the finding's whitespace-normalized source line together with the nearest "
    f"{ANCHOR_CONTEXT} non-blank lines above and below it, then '#' and the 1-based ordinal of that window among "
    "identical windows in the file"
)


def fail(diagnostics: list[str]) -> int:
    for diagnostic in diagnostics:
        print(diagnostic, file=sys.stderr)
    return 1 if diagnostics else 0


def workspace_members(root: Path) -> list[str]:
    manifest = tomllib.loads((root / "Cargo.toml").read_text())
    names = []
    for member in manifest["workspace"]["members"]:
        member_manifest = tomllib.loads((root / member / "Cargo.toml").read_text())
        names.append(member_manifest["package"]["name"])
    return sorted(names)


def nextest_scope(root: Path) -> list[str]:
    config = tomllib.loads((root / ".config/nextest.toml").read_text())
    diagnostics = []
    for profile in AGGREGATE_PROFILES:
        expression = config.get("profile", {}).get(profile, {}).get("default-filter", "")
        selected = set(re.findall(r"package\(([^)]+)\)", expression))
        for member in workspace_members(root):
            if member not in selected:
                diagnostics.append(f"nextest-profile-missing-member:{profile}:{member}")
    return diagnostics


def junit_members(junit: Path, members: list[str]) -> list[str]:
    executed: dict[str, int] = {}
    for suite in ElementTree.parse(junit).getroot().iter("testsuite"):
        member = suite.get("name", "").split("::", 1)[0]
        ran = sum(1 for case in suite.iter("testcase") if case.find("skipped") is None)
        executed[member] = executed.get(member, 0) + ran
    return [f"workspace-member-executed-no-tests:{member}" for member in members if not executed.get(member)]


def rust_files(root: Path):
    for top in RUST_SOURCE_ROOTS:
        base = root / top
        if base.is_dir():
            yield from sorted(path for path in base.rglob("*.rs") if "target" not in path.parts)


def ignored_tests(root: Path) -> tuple[dict[tuple[str, str], str], list[str]]:
    """Map (path, function) to the ignore reason; diagnose attributes without a reason or function."""
    found: dict[tuple[str, str], str] = {}
    diagnostics = []
    for path in rust_files(root):
        relative = path.relative_to(root).as_posix()
        lines = path.read_text(errors="replace").splitlines()
        for index, line in enumerate(lines):
            match = IGNORE_ATTRIBUTE.search(line)
            if not match or line.lstrip().startswith("//"):
                continue
            reason = IGNORE_REASON.match(match.group("rest"))
            function = next(
                (hit.group("name") for later in lines[index + 1 : index + 8] if (hit := FUNCTION.search(later))),
                None,
            )
            if function is None:
                diagnostics.append(f"ignored-test-without-function:{relative}:{index + 1}")
                continue
            if reason is None:
                diagnostics.append(f"ignored-test-without-reason:{relative}::{function}")
            found[(relative, function)] = reason.group("reason") if reason else ""
    return found, diagnostics


def nix_string_end(text: str, index: int, close: str) -> int:
    """Index just past a Nix string whose body starts at `index`, skipping escapes and antiquotations."""
    while index < len(text):
        if close == '"' and text[index] == "\\":
            index += 2
        elif close == '"' and text[index] == '"':
            return index + 1
        elif close == "''" and text.startswith("''\\", index):
            index += 4
        elif close == "''" and (text.startswith("'''", index) or text.startswith("''$", index)):
            index += 3
        elif close == "''" and text.startswith("''", index):
            return index + 2
        elif text.startswith("${", index):
            index = nix_scan(text, index + 2, "}") + 1
        else:
            index += 1
    return len(text)


def nix_scan(text: str, index: int, stop: str) -> int:
    """Index of `stop` at bracket depth 0 from `index`, outside strings and comments (or the text length)."""
    depth = 0
    while index < len(text):
        if text[index] == "#":
            newline = text.find("\n", index)
            index = len(text) if newline < 0 else newline
            continue
        if text.startswith("/*", index):
            close = text.find("*/", index + 2)
            index = len(text) if close < 0 else close + 2
            continue
        if text[index] == '"':
            index = nix_string_end(text, index + 1, '"')
            continue
        if text.startswith("''", index):
            index = nix_string_end(text, index + 2, "''")
            continue
        if depth == 0 and text[index] == stop:
            return index
        depth += (text[index] in "([{") - (text[index] in ")]}")
        index += 1
    return len(text)


def nix_bindings(text: str, name: str) -> list[str]:
    """Every expression bound to the attribute or let name `name` in `text`."""
    expressions = []
    for match in re.finditer(rf"^[ \t]*{re.escape(name)}[ \t]*=(?!=)", text, re.MULTILINE):
        expressions.append(text[match.end() : nix_scan(text, match.end(), ";")])
    return expressions


def nix_rail_definition(root: Path, flake: str, name: str, seen: set[str]) -> str | None:
    """The rail's own binding, the Nix files it imports, and the bindings it aliases; None if absent or ambiguous."""
    bindings = nix_bindings(flake, name)
    if len(bindings) != 1:
        return None
    expression = bindings[0]
    parts = [expression]
    for match in NIX_IMPORTED_FILE.finditer(expression):
        path = root / match.group("path")
        if path.is_file():
            parts.append(path.read_text())
    alias = expression.strip()
    if NIX_IDENTIFIER.match(alias) and alias not in seen:
        seen.add(alias)
        aliased = nix_rail_definition(root, flake, alias, seen)
        if aliased is not None:
            parts.append(aliased)
    return "\n".join(parts)


def rail_selection(root: Path, rail: str, selector: str) -> str | None:
    """Diagnostic suffix when the named rail's own definition does not select `selector`; None when it does."""
    if rail.startswith("nix:"):
        name = rail.removeprefix("nix:")
        definition = nix_rail_definition(root, (root / "flake.nix").read_text(), name, {name})
        if definition is None:
            return "rail-missing"
        if f"ignored-rail-evidence --rail {rail}" not in definition:
            return "rail-without-evidence"
        return None if selector in definition else "rail-does-not-select"
    path = root / ".github/workflows" / rail.removeprefix("workflow:")
    if not path.is_file():
        return "rail-missing"
    for line in path.read_text().splitlines():
        words = line.split()
        if WORKFLOW_TEST_COMMAND.search(line) and "--ignored" in words and selector in words:
            return None
    return "rail-does-not-select"


def audit_entry(root: Path, entry: dict, label: str) -> list[str]:
    category = entry.get("category", "")
    disposition = entry.get("disposition", "")
    allowed = CATEGORY_DISPOSITIONS.get(category)
    if allowed is None:
        return [f"ignored-test-unknown-category:{label}:{category or '<missing>'}"]
    if disposition not in allowed:
        return [f"ignored-test-disposition-not-allowed:{label}:{category}:{disposition or '<missing>'}"]
    diagnostics = []
    if not entry.get("prerequisite", "").strip():
        diagnostics.append(f"ignored-test-without-prerequisite:{label}")
    if disposition != "rail":
        return diagnostics
    rail = entry.get("rail", "")
    selector = entry.get("selector", "")
    if not rail.startswith(RAIL_KINDS):
        return [*diagnostics, f"ignored-test-rail-missing:{label}:{rail or '<missing>'}"]
    if not selector:
        return [*diagnostics, f"ignored-test-rail-does-not-select:{label}:{rail}:<missing>"]
    problem = rail_selection(root, rail, selector)
    if problem is not None:
        suffix = f"{rail}:{selector}" if problem == "rail-does-not-select" else rail
        return [*diagnostics, f"ignored-test-{problem}:{label}:{suffix}"]
    if not label.rsplit("::", 1)[-1].startswith(selector.rsplit("::", 1)[-1]):
        diagnostics.append(f"ignored-test-selector-mismatch:{label}:{selector}")
    return diagnostics


def audit_ignored(root: Path, inventory_path: Path) -> list[str]:
    found, diagnostics = ignored_tests(root)
    inventory = tomllib.loads(inventory_path.read_text()).get("test", [])
    listed = set()
    for entry in inventory:
        key = (entry.get("path", ""), entry.get("test", ""))
        label = f"{key[0]}::{key[1]}"
        if key in listed:
            diagnostics.append(f"ignored-inventory-duplicate:{label}")
        listed.add(key)
        if key not in found:
            diagnostics.append(f"ignored-inventory-stale:{label}")
            continue
        diagnostics += audit_entry(root, entry, label)
    for key in sorted(set(found) - listed):
        diagnostics.append(f"ignored-test-unaccounted:{key[0]}::{key[1]}")
    return diagnostics


def libtest_passed(text: str) -> set[str]:
    """Names libtest reported `ok`, including `--nocapture` runs whose output splits the status line."""
    passed: set[str] = set()
    pending = None
    for line in text.splitlines():
        match = LIBTEST_CASE.match(line)
        if match:
            rest = match.group("rest").strip()
            pending = None if rest == "ok" or rest == "FAILED" or rest.startswith("ignored") else match.group("name")
            if rest == "ok":
                passed.add(match.group("name"))
        elif pending is not None and line.strip() == "ok":
            passed.add(pending)
            pending = None
    return passed


def rail_evidence(inventory_path: Path, rail: str, logs: list[Path]) -> list[str]:
    inventory = tomllib.loads(inventory_path.read_text()).get("test", [])
    entries = [entry for entry in inventory if entry.get("disposition") == "rail" and entry.get("rail") == rail]
    if not entries:
        return [f"ignored-rail-has-no-entries:{rail}"]
    passed: set[str] = set()
    for log in logs:
        passed |= libtest_passed(log.read_text(errors="replace"))
    diagnostics = []
    for entry in entries:
        test, selector = entry.get("test", ""), entry.get("selector", "")
        ran = any((name == test or name.endswith(f"::{test}")) and selector in name for name in passed)
        if not test or not selector or not ran:
            diagnostics.append(f"ignored-rail-test-not-passed:{rail}:{entry.get('path', '')}::{test}")
    return diagnostics


def list_tests(executable: str, cwd: Path) -> list[str] | None:
    """Test names (ignored ones included) a libtest binary lists; None if it cannot list them."""
    result = subprocess.run(
        [executable, "--list", "--format", "terse"], cwd=cwd, capture_output=True, text=True, check=False
    )
    if result.returncode != 0:
        return None
    return sorted(line.removesuffix(": test") for line in result.stdout.splitlines() if line.endswith(": test"))


def package_targets(metadata: Path, package: str) -> tuple[str, list[dict]]:
    """Manifest path and Cargo targets of `package` from `cargo metadata --no-deps` output."""
    document = json.loads(metadata.read_text())
    for candidate in document["packages"]:
        if candidate["name"] == package:
            return candidate["manifest_path"], candidate["targets"]
    raise ValueError(f"package {package} missing from cargo metadata")


def statically_gated(targets: list[dict]) -> dict[str, str]:
    """Integration test targets gated by `required-features` or a crate-level feature `cfg`."""
    gated = {}
    for target in targets:
        if "test" not in target["kind"]:
            continue
        if target.get("required-features"):
            gated[target["name"]] = "required-features"
            continue
        if FEATURE_CRATE_CFG.search(Path(target["src_path"]).read_text(errors="replace")):
            gated[target["name"]] = "crate-cfg"
    return gated


def test_executables(messages: Path, manifest: str) -> list[tuple[str, str, str]]:
    """(kind, target name, executable) of every test-profile artifact Cargo built for `manifest`."""
    built = []
    for line in messages.read_text().splitlines():
        if not line.startswith("{"):
            continue
        message = json.loads(line)
        if message.get("reason") != "compiler-artifact" or message.get("manifest_path") != manifest:
            continue
        if not message.get("executable") or not message.get("profile", {}).get("test"):
            continue
        built.append((message["target"]["kind"][0], message["target"]["name"], message["executable"]))
    return sorted(built)


def feature_classification(path: Path) -> tuple[dict[str, dict], list[str]]:
    entries: dict[str, dict] = {}
    diagnostics = []
    for entry in tomllib.loads(path.read_text()).get("target", []):
        label = entry.get("target", "")
        if label in entries:
            diagnostics.append(f"feature-gated-classification-duplicate:{label}")
        if entry.get("disposition") not in FEATURE_DISPOSITIONS:
            diagnostics.append(f"feature-gated-classification-disposition:{label}:{entry.get('disposition', '<missing>')}")
        if not entry.get("reason", "").strip():
            diagnostics.append(f"feature-gated-classification-without-reason:{label}")
        entries[label] = entry
    return entries, diagnostics


def default_test_names(defaults: Path, cwd: Path) -> tuple[dict[str, set[str]], list[str]]:
    """Per default-feature binary, the tests it lists; a binary that cannot list fails closed."""
    lists: dict[str, set[str]] = {}
    diagnostics = []
    for binary in sorted(path for path in defaults.iterdir() if path.is_file() and os.access(path, os.X_OK)):
        names = list_tests(str(binary), cwd)
        if names is None:
            diagnostics.append(f"feature-tests-default-list-failed:{binary.name}")
        lists[binary.name] = set(names or [])
    return lists, diagnostics


def run_gated(executable: str, names: list[str], cwd: Path, log: Path) -> dict[str, int] | None:
    """Run exactly `names`; the parsed libtest result, or None when the binary failed or printed none."""
    result = subprocess.run(
        [executable, "--exact", *names], cwd=cwd, capture_output=True, text=True, check=False
    )
    log.write_text(result.stdout + result.stderr)
    print(result.stdout, end="")
    summary = LIBTEST_RESULT.search(result.stdout)
    if result.returncode != 0 or summary is None or summary.group("status") != "ok":
        return None
    return {key: int(summary.group(key)) for key in ("passed", "failed", "ignored")}


def gated_target_decision(label: str, result: dict[str, int] | None, classification: dict | None) -> list[str]:
    if result is None:
        return [f"feature-gated-tests-failed:{label}"]
    disposition = (classification or {}).get("disposition")
    if result["passed"] > 0:
        return [f"feature-gated-classification-stale:{label}"] if classification else []
    if disposition == "ignored-inventory" and result["ignored"] > 0:
        return []
    return [f"feature-gated-target-executed-no-tests:{label}"]


def feature_tests(arguments: argparse.Namespace) -> list[str]:
    root = arguments.root
    manifest, targets = package_targets(arguments.metadata, arguments.package)
    static = statically_gated(targets)
    classified, diagnostics = feature_classification(root / arguments.classification)
    defaults, problems = default_test_names(arguments.default_binaries, root)
    diagnostics += problems
    every_default = set().union(*defaults.values())
    arguments.out.mkdir(parents=True, exist_ok=True)
    summary, seen, built = [], set(), set()
    for kind, name, executable in test_executables(arguments.artifacts, manifest):
        label = f"{kind}:{name}"
        built.add(name if kind == "test" else label)
        union = list_tests(executable, root)
        if union is None:
            diagnostics.append(f"feature-tests-list-failed:{label}")
            continue
        baseline = defaults.get(name, set()) if kind == "test" else every_default
        gated = sorted(set(union) - baseline)
        if kind == "test" and name in static and not union:
            diagnostics.append(f"feature-gated-target-lists-no-tests:{label}")
        if not gated:
            continue
        seen.add(label)
        classification = classified.get(label)
        if classification and classification.get("disposition") == "environment-blocked":
            summary.append(f"{label} gated={len(gated)} environment-blocked")
            continue
        result = run_gated(executable, gated, root, arguments.out / f"{kind}-{name}.txt")
        diagnostics += gated_target_decision(label, result, classification)
        summary.append(f"{label} gated={len(gated)} result={result}")
    diagnostics += [f"feature-gated-target-not-built:test:{name}" for name in sorted(set(static) - built)]
    diagnostics += [f"feature-gated-classification-stale:{label}" for label in sorted(set(classified) - seen)]
    if not seen:
        diagnostics.append("feature-matrix-no-feature-gated-tests")
    (arguments.out / "summary.txt").write_text("".join(f"{line}\n" for line in summary))
    return diagnostics


def hegel_properties(root: Path) -> list[str]:
    names = []
    for path in sorted((root / HEGEL_SOURCE_ROOT).rglob("*.rs")):
        lines = path.read_text(errors="replace").splitlines()
        for index, line in enumerate(lines):
            if HEGEL_ATTRIBUTE.search(line) and not line.lstrip().startswith("//"):
                function = next(
                    (hit.group("name") for later in lines[index + 1 : index + 6] if (hit := FUNCTION.search(later))),
                    None,
                )
                if function:
                    names.append(function)
    return sorted(set(names))


def hegel_coverage(root: Path, junit: Path | None) -> list[str]:
    names = hegel_properties(root)
    if not names:
        return [f"hegel-properties-missing:{HEGEL_SOURCE_ROOT}"]
    if junit is None:
        return []
    executed: dict[str, bool] = {}
    for suite in ElementTree.parse(junit).getroot().iter("testsuite"):
        if not suite.get("name", "").startswith(HEGEL_PACKAGE):
            continue
        for case in suite.iter("testcase"):
            passed = case.find("failure") is None and case.find("error") is None and case.find("skipped") is None
            executed[case.get("name", "").rsplit("::", 1)[-1]] = passed
    diagnostics = []
    for name in names:
        if name not in executed:
            diagnostics.append(f"hegel-property-not-executed:{name}")
        elif not executed[name]:
            diagnostics.append(f"hegel-property-failed:{name}")
    return diagnostics


def file_anchors(path: Path) -> list[str]:
    """Anchor of every line: a hash of the line and its non-blank neighbours, plus the window's ordinal."""
    lines = [" ".join(line.split()) for line in path.read_text(errors="replace").splitlines()] or [""]
    non_blank = [index for index, text in enumerate(lines) if text]
    anchors = []
    seen: dict[str, int] = {}
    for index, text in enumerate(lines):
        position = bisect.bisect_left(non_blank, index)
        above = non_blank[max(position - ANCHOR_CONTEXT, 0) : position]
        below = non_blank[bisect.bisect_right(non_blank, index) :][:ANCHOR_CONTEXT]
        window = [lines[other] for other in above] + [text] + [lines[other] for other in below]
        digest = hashlib.sha256("\n".join(window).encode()).hexdigest()[:16]
        seen[digest] = seen.get(digest, 0) + 1
        anchors.append(f"sha256:{digest}#{seen[digest]}")
    return anchors


def finding_anchor(root: Path, file: str, line: int, cache: dict[str, list[str] | None] | None = None) -> str | None:
    cache = {} if cache is None else cache
    if file not in cache:
        path = root / file
        cache[file] = file_anchors(path) if path.is_file() else None
    anchors = cache[file]
    if anchors is None or not 1 <= line <= len(anchors):
        return None
    return anchors[line - 1]


Identity = tuple[str, str, str, str]


def octet_identities(root: Path, summary: Path) -> tuple[dict[Identity, int], int, list[str]]:
    """Count each (crate, file, lint, anchor) identity in the summary index; also the index size."""
    counts: dict[Identity, int] = {}
    cache: dict[str, list[str] | None] = {}
    diagnostics = []
    indexed = 0
    in_index = False
    for line in summary.read_text().splitlines():
        if line.strip() == "Index:":
            in_index = True
            continue
        if not in_index or not line.strip():
            continue
        if line.strip() == "---":
            break
        match = OCTET_INDEX_LINE.match(line)
        if match is None:
            raise ValueError(f"unparsed Octet index line: {line!r}")
        indexed += 1
        crate, file, lint = match.group("crate"), match.group("file"), match.group("lint")
        anchor = finding_anchor(root, file, int(match.group("line")), cache)
        if anchor is None:
            diagnostics.append(f"octet-finding-unanchored:{crate}:{file}:{match.group('line')}:{lint}")
            continue
        counts[(crate, file, lint, anchor)] = counts.get((crate, file, lint, anchor), 0) + 1
    return counts, indexed, diagnostics


def octet_status(artifacts: Path, indexed: int, scope: list[str]) -> list[str]:
    diagnostics = []
    status = json.loads((artifacts / "status.json").read_text())
    if status.get("cargo_process_exit", {}).get("classification") != "success":
        diagnostics.append("octet-cargo-process-failed")
    if status.get("total_findings") != indexed:
        diagnostics.append(f"octet-index-incomplete:{status.get('total_findings')}!={indexed}")
    command = (artifacts / "command.txt").read_text().split()
    selected = {command[index + 1] for index, word in enumerate(command[:-1]) if word in ("-p", "--package")}
    for member in scope:
        if member not in selected:
            diagnostics.append(f"octet-scope-missing-member:{member}")
    return diagnostics


def baseline_identities(baseline: Path) -> tuple[dict[Identity, int], int, list[str]]:
    """Recorded identity counts and `total`; diagnoses schema, duplicate rows, bad counts, and total drift."""
    document = json.loads(baseline.read_text())
    if document.get("schema") != BASELINE_SCHEMA:
        return {}, 0, [f"octet-baseline-schema:{document.get('schema')}!={BASELINE_SCHEMA}"]
    recorded: dict[Identity, int] = {}
    diagnostics = []
    for row in document.get("findings", []):
        key = (row.get("crate", ""), row.get("file", ""), row.get("lint", ""), row.get("anchor", ""))
        count = row.get("count")
        if key in recorded:
            diagnostics.append(f"octet-baseline-duplicate-row:{':'.join(key)}")
        if not isinstance(count, int) or isinstance(count, bool) or count < 1 or not all(key):
            diagnostics.append(f"octet-baseline-invalid-row:{':'.join(key)}:{count}")
            continue
        recorded[key] = recorded.get(key, 0) + count
    total = document.get("total")
    if total != sum(recorded.values()):
        diagnostics.append(f"octet-baseline-total-mismatch:{total}!={sum(recorded.values())}")
    return recorded, total if isinstance(total, int) else -1, diagnostics


def octet_check(root: Path, artifacts: Path, baseline: Path, scope: list[str], crates: list[str]) -> list[str]:
    counts, indexed, diagnostics = octet_identities(root, artifacts / "summary.txt")
    diagnostics += octet_status(artifacts, indexed, scope)
    expected, total, problems = baseline_identities(baseline)
    diagnostics += problems
    if crates:
        counts = {key: value for key, value in counts.items() if key[0] in crates}
        expected = {key: value for key, value in expected.items() if key[0] in crates}
    elif not problems and total != indexed:
        diagnostics.append(f"octet-baseline-total-mismatch:{total}!={indexed}")
    for key in sorted(expected):
        if key[0] in octet_zero_crates(root):
            diagnostics.append(f"octet-baseline-admits-zero-debt-crate:{':'.join(key)}")
    for key in sorted(set(counts) | set(expected)):
        actual, recorded = counts.get(key, 0), expected.get(key, 0)
        if actual > recorded:
            diagnostics.append(f"octet-new-finding:{':'.join(key)}:{recorded}->{actual}")
        elif actual < recorded:
            diagnostics.append(f"octet-baseline-stale:{':'.join(key)}:{recorded}->{actual}")
    return diagnostics


def octet_write(root: Path, artifacts: Path, baseline: Path, scope: list[str]) -> list[str]:
    counts, indexed, diagnostics = octet_identities(root, artifacts / "summary.txt")
    diagnostics += octet_status(artifacts, indexed, scope)
    zero_crates = octet_zero_crates(root)
    diagnostics += [
        f"octet-zero-debt-crate-has-findings:{key[0]}:{key[1]}:{key[2]}" for key in counts if key[0] in zero_crates
    ]
    if diagnostics:
        return diagnostics
    rows = [
        {"crate": crate, "file": file, "lint": lint, "anchor": anchor, "count": count}
        for (crate, file, lint, anchor), count in sorted(counts.items())
    ]
    document = {"schema": BASELINE_SCHEMA, "anchor": ANCHOR_DESCRIPTION, "total": indexed, "findings": rows}
    baseline.parent.mkdir(parents=True, exist_ok=True)
    baseline.write_text(json.dumps(document, indent=1) + "\n")
    return []


def parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", type=Path, default=Path("."))
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("nextest-scope")
    members = commands.add_parser("junit-members")
    members.add_argument("--junit", type=Path, required=True)
    members.add_argument("--member", action="append", required=True)
    inventory = Path("config/testing/ignored-tests.toml")
    ignored = commands.add_parser("ignored-tests")
    ignored.add_argument("--inventory", type=Path, default=inventory)
    evidence = commands.add_parser("ignored-rail-evidence")
    evidence.add_argument("--inventory", type=Path, default=inventory)
    evidence.add_argument("--rail", required=True)
    evidence.add_argument("--log", type=Path, action="append", required=True)
    features = commands.add_parser("feature-tests")
    features.add_argument("--metadata", type=Path, required=True, help="cargo metadata --no-deps output")
    features.add_argument("--artifacts", type=Path, required=True, help="cargo test --no-run JSON messages")
    features.add_argument("--default-binaries", type=Path, required=True, help="default-feature test binaries")
    features.add_argument("--package", default="molten")
    features.add_argument("--classification", type=Path, default=Path("config/testing/feature-gated-targets.toml"))
    features.add_argument("--out", type=Path, required=True)
    hegel = commands.add_parser("hegel-coverage")
    hegel.add_argument("--junit", type=Path)
    octet = commands.add_parser("octet-baseline")
    octet.add_argument("mode", choices=("check", "write"))
    octet.add_argument("--artifacts", type=Path, required=True)
    octet.add_argument("--baseline", type=Path, default=Path("config/testing/octet-workspace-baseline.json"))
    octet.add_argument("--scope", default="", help="comma-separated packages the run must select with -p")
    octet.add_argument("--crate", action="append", default=[], help="compare only these Octet crate names")
    return parser


def main(argv: list[str]) -> int:
    arguments = parser().parse_args(argv)
    root = arguments.root
    if arguments.command == "nextest-scope":
        return fail(nextest_scope(root))
    if arguments.command == "junit-members":
        return fail(junit_members(arguments.junit, arguments.member))
    if arguments.command == "ignored-tests":
        return fail(audit_ignored(root, root / arguments.inventory))
    if arguments.command == "ignored-rail-evidence":
        return fail(rail_evidence(root / arguments.inventory, arguments.rail, arguments.log))
    if arguments.command == "feature-tests":
        return fail(feature_tests(arguments))
    if arguments.command == "hegel-coverage":
        return fail(hegel_coverage(root, arguments.junit))
    scope = [member for member in arguments.scope.split(",") if member]
    if arguments.mode == "write":
        return fail(octet_write(root, arguments.artifacts, root / arguments.baseline, scope))
    return fail(octet_check(root, arguments.artifacts, root / arguments.baseline, scope, arguments.crate))


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
