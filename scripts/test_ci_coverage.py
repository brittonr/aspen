#!/usr/bin/env python3
"""Positive and negative checks for scripts/ci-coverage.py (no network, temporary fixtures only).

r[verify molten.testing.ci_workspace_member_coverage]
r[verify molten.testing.feature_and_ignore_ci_accounting]
r[verify molten.testing.molten_core_hegel_properties]
r[verify molten.evidence_gates.ci_octet_and_cairn_hermetic]
"""
import contextlib
import importlib.util
import io
import json
import sys
import tempfile
import textwrap
import unittest
from pathlib import Path

sys.dont_write_bytecode = True
SCRIPT = Path(__file__).with_name("ci-coverage.py")
SPEC = importlib.util.spec_from_file_location("ci_coverage", SCRIPT)
coverage = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(coverage)

MEMBERS = ("molten", "molten-core")
FULL_FILTER = "package(molten) | package(molten-core)"
SUMMARY_HEADER = "--- octet summary ---\nFindings: {total}\n\nIndex:\n"
EVIDENCE = "python3 scripts/ci-coverage.py ignored-rail-evidence --rail nix:{rail} --log run.txt"
FLAKE = f"""\
{{
  checks = {{
    some-rail = run ''
      "$binary" --ignored selected_case | tee run.txt
      {EVIDENCE.format(rail="some-rail")}
    '';
    other-rail = run "stale_case";
    aliased-rail = aliasedRail;
  }};
  aliasedRail = import ./nix/aliased.nix {{ inherit pkgs; }};
  unused = "never_run_case";
}}
"""
RESULT_OK = "test result: ok. {passed} passed; 0 failed; {ignored} ignored; 0 measured; 0 filtered out"


def write(root: Path, relative: str, text: str) -> None:
    path = root / relative
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(textwrap.dedent(text))


def run(*argv: str) -> tuple[int, list[str]]:
    stderr = io.StringIO()
    with contextlib.redirect_stderr(stderr), contextlib.redirect_stdout(io.StringIO()):
        status = coverage.main(list(argv))
    return status, stderr.getvalue().splitlines()


class Fixture(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.root = Path(self.directory.name)
        write(
            self.root,
            "Cargo.toml",
            '[workspace]\nmembers = [".", "crates/molten-core"]\n'
            '[workspace.metadata.octet]\ndefault_scope = ["-p", "molten"]\n'
            '[package]\nname = "molten"\n',
        )
        write(self.root, "crates/molten-core/Cargo.toml", '[package]\nname = "molten-core"\n')
        (self.root / "flake.nix").write_text(FLAKE)

    def tearDown(self) -> None:
        self.directory.cleanup()

    def nextest(self, ci: str, deterministic: str) -> None:
        write(
            self.root,
            ".config/nextest.toml",
            f"[profile.ci]\ndefault-filter = '{ci}'\n[profile.deterministic]\ndefault-filter = '({deterministic}) & not test(/vm/)'\n",
        )


class NextestScopeTests(Fixture):
    def test_every_member_selected_passes(self):
        self.nextest(FULL_FILTER, FULL_FILTER)
        self.assertEqual(run("--root", str(self.root), "nextest-scope"), (0, []))

    def test_dropped_member_is_reported_per_profile(self):
        self.nextest("package(molten)", FULL_FILTER)
        self.assertEqual(
            run("--root", str(self.root), "nextest-scope"), (1, ["nextest-profile-missing-member:ci:molten-core"])
        )


class JunitMemberTests(Fixture):
    def junit(self, suites: dict[str, str]) -> Path:
        cases = "".join(f'<testsuite name="{name}">{body}</testsuite>' for name, body in suites.items())
        path = self.root / "junit.xml"
        path.write_text(f"<testsuites>{cases}</testsuites>")
        return path

    def members(self, junit: Path) -> tuple[int, list[str]]:
        return run("junit-members", "--junit", str(junit), "--member", "molten", "--member", "molten-core")

    def test_members_with_executed_tests_pass(self):
        ran = '<testcase name="t"/>'
        junit = self.junit({"molten::nix-test/0/a": ran * 2, "molten-core::nix-test/0/b": ran})
        self.assertEqual(self.members(junit), (0, []))

    def test_member_without_executed_tests_denies(self):
        junit = self.junit({"molten::nix-test/0/a": '<testcase name="t"/>', "molten-core::nix-test/0/b": ""})
        self.assertEqual(self.members(junit), (1, ["workspace-member-executed-no-tests:molten-core"]))

    def test_member_with_only_skipped_tests_denies(self):
        skipped = '<testcase name="t"><skipped/></testcase>'
        junit = self.junit({"molten::nix-test/0/a": '<testcase name="t"/>', "molten-core::nix-test/0/b": skipped * 3})
        self.assertEqual(self.members(junit), (1, ["workspace-member-executed-no-tests:molten-core"]))


class IgnoredTestAuditTests(Fixture):
    def source(self, attribute: str) -> None:
        write(self.root, "src/lib.rs", f"#[test]\n{attribute}\nfn selected_case() {{}}\n")

    def inventory(self, **entry: str) -> None:
        fields = {"path": "src/lib.rs", "test": "selected_case", **entry}
        body = "".join(f'{key} = "{value}"\n' for key, value in fields.items())
        write(self.root, "config/testing/ignored-tests.toml", f"[[test]]\n{body}")

    def railed(self, rail: str, selector: str = "selected_case") -> None:
        self.source('#[ignore = "needs tools"]')
        self.inventory(category="pinned-host-tools", disposition="rail", rail=rail, selector=selector, prerequisite="t")

    def audit(self) -> tuple[int, list[str]]:
        return run("--root", str(self.root), "ignored-tests")

    def test_railed_same_host_test_passes(self):
        self.railed("nix:some-rail")
        self.assertEqual(self.audit(), (0, []))

    def test_aliased_rail_resolves_its_imported_file(self):
        write(self.root, "nix/aliased.nix", f"run ''\n  binary --ignored selected_case\n  {EVIDENCE.format(rail='aliased-rail')}\n''\n")
        self.railed("nix:aliased-rail")
        self.assertEqual(self.audit(), (0, []))

    def test_environment_blocked_test_passes(self):
        self.source('#[ignore = "needs device"]')
        self.inventory(category="nvme-device", disposition="environment-blocked", prerequisite="physical NVMe")
        self.assertEqual(self.audit(), (0, []))

    def test_unlisted_and_unjustified_ignored_test_denies(self):
        self.source("#[ignore]")
        write(self.root, "config/testing/ignored-tests.toml", "")
        self.assertEqual(
            self.audit(),
            (1, ["ignored-test-without-reason:src/lib.rs::selected_case", "ignored-test-unaccounted:src/lib.rs::selected_case"]),
        )

    def test_same_host_test_cannot_be_marked_environment_blocked(self):
        self.source('#[ignore = "needs tools"]')
        self.inventory(category="pinned-host-tools", disposition="environment-blocked", prerequisite="tools")
        self.assertEqual(
            self.audit(),
            (1, ["ignored-test-disposition-not-allowed:src/lib.rs::selected_case:pinned-host-tools:environment-blocked"]),
        )

    def test_missing_rail_and_unselected_test_deny(self):
        self.railed("nix:absent-rail")
        self.assertEqual(self.audit(), (1, ["ignored-test-rail-missing:src/lib.rs::selected_case:nix:absent-rail"]))
        self.railed("nix:some-rail", "other_case")
        self.assertEqual(
            self.audit(), (1, ["ignored-test-rail-does-not-select:src/lib.rs::selected_case:nix:some-rail:other_case"])
        )

    def test_stale_selector_left_in_another_rail_or_constant_denies(self):
        # The selector text survives elsewhere in flake.nix, but the named rail no longer selects it.
        self.railed("nix:some-rail", "never_run_case")
        self.assertEqual(
            self.audit(),
            (1, ["ignored-test-rail-does-not-select:src/lib.rs::selected_case:nix:some-rail:never_run_case"]),
        )

    def test_rail_without_execution_evidence_denies(self):
        self.railed("nix:other-rail", "stale_case")
        self.assertEqual(self.audit(), (1, ["ignored-test-rail-without-evidence:src/lib.rs::selected_case:nix:other-rail"]))

    def test_workflow_rail_must_pass_selector_to_an_ignored_test_command(self):
        workflow = ".github/workflows/live.yml"
        write(self.root, workflow, "run: |\n  cargo test --locked -p molten --lib selected_case -- --ignored\n")
        self.railed("workflow:live.yml")
        self.assertEqual(self.audit(), (0, []))
        write(self.root, workflow, "run: |\n  echo selected_case\n  cargo test --locked -p molten --lib other -- --ignored\n")
        self.assertEqual(
            self.audit(), (1, ["ignored-test-rail-does-not-select:src/lib.rs::selected_case:workflow:live.yml:selected_case"])
        )

    def test_stale_entry_and_unknown_category_deny(self):
        self.source("")
        self.inventory(category="unknown", disposition="rail", prerequisite="x")
        self.assertEqual(self.audit(), (1, ["ignored-inventory-stale:src/lib.rs::selected_case"]))
        self.source('#[ignore = "x"]')
        self.assertEqual(self.audit(), (1, ["ignored-test-unknown-category:src/lib.rs::selected_case:unknown"]))


class IgnoredRailEvidenceTests(Fixture):
    def setUp(self) -> None:
        super().setUp()
        body = 'path = "src/lib.rs"\ntest = "selected_case"\ndisposition = "rail"\nrail = "nix:some-rail"\nselector = "selected_case"\n'
        write(self.root, "config/testing/ignored-tests.toml", f"[[test]]\n{body}")

    def evidence(self, log: str, rail: str = "nix:some-rail") -> tuple[int, list[str]]:
        path = self.root / "run.txt"
        path.write_text(log)
        return run("--root", str(self.root), "ignored-rail-evidence", "--rail", rail, "--log", str(path))

    def test_passed_test_in_rail_output_passes(self):
        self.assertEqual(self.evidence("running 1 test\ntest module::selected_case ... ok\n"), (0, []))

    def test_nocapture_output_between_name_and_status_passes(self):
        self.assertEqual(self.evidence("test module::selected_case ... consumer log line\nmore output\nok\n"), (0, []))

    def test_stale_selector_that_the_rail_no_longer_runs_denies(self):
        denied = (1, ["ignored-rail-test-not-passed:nix:some-rail:src/lib.rs::selected_case"])
        self.assertEqual(self.evidence("test module::other_case ... ok\n"), denied)
        self.assertEqual(self.evidence("test module::selected_case ... FAILED\n"), denied)
        self.assertEqual(self.evidence("test module::selected_case ... ignored\n"), denied)

    def test_rail_without_inventory_entries_denies(self):
        self.assertEqual(self.evidence("", "nix:other-rail"), (1, ["ignored-rail-has-no-entries:nix:other-rail"]))


class FeatureTestsTests(Fixture):
    """`feature-tests` against shell stand-ins for libtest binaries."""

    def binary(self, relative: str, listed: list[str], output: str, status: int = 0) -> str:
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        listing = "".join(f"{name}: test\\n" for name in listed)
        path.write_text(
            "#!/bin/sh\n"
            f'if [ "$1" = "--list" ]; then printf "{listing}"; exit 0; fi\n'
            f'printf "%s\\n" "$*" > "{path}.args"\n'
            f"printf '%s\\n' '{output}'\n"
            f"exit {status}\n"
        )
        path.chmod(0o755)
        return str(path)

    def build(self, union: dict[tuple[str, str], tuple[list[str], str, int]], static: dict[str, str]) -> None:
        targets = [{"name": "molten", "kind": ["lib"], "src_path": str(self.root / "src/lib.rs")}]
        write(self.root, "src/lib.rs", "")
        for name, gate in static.items():
            write(self.root, f"tests/{name}.rs", gate)
            targets.append({"name": name, "kind": ["test"], "src_path": str(self.root / f"tests/{name}.rs")})
        manifest = str(self.root / "Cargo.toml")
        package = {"name": "molten", "manifest_path": manifest, "targets": targets}
        (self.root / "metadata.json").write_text(json.dumps({"packages": [package]}))
        messages = []
        for (kind, name), (listed, output, status) in union.items():
            executable = self.binary(f"union/{kind}-{name}", listed, output, status)
            target = {"kind": [kind], "name": name}
            messages.append(
                {"reason": "compiler-artifact", "manifest_path": manifest, "target": target, "profile": {"test": True}, "executable": executable}
            )
        (self.root / "artifacts.json").write_text("".join(json.dumps(message) + "\n" for message in messages))
        self.binary("defaults/molten-0123456789", ["default_unit"], RESULT_OK.format(passed=1, ignored=0))
        self.binary("defaults/componentrpc", [], RESULT_OK.format(passed=0, ignored=0))

    def classify(self, *entries: str) -> None:
        write(self.root, "config/testing/feature-gated-targets.toml", "".join(f"[[target]]\n{entry}\n" for entry in entries))

    def features(self) -> tuple[int, list[str]]:
        return run(
            "--root", str(self.root), "feature-tests", "--metadata", str(self.root / "metadata.json"),
            "--artifacts", str(self.root / "artifacts.json"), "--default-binaries", str(self.root / "defaults"),
            "--out", str(self.root / "out"),
        )

    def standard(self, rpc_output: str, rpc_status: int = 0) -> None:
        lib = (["default_unit", "gated_unit"], RESULT_OK.format(passed=1, ignored=0), 0)
        self.build(
            {("lib", "molten"): lib, ("test", "componentrpc"): (["rpc_roundtrip"], rpc_output, rpc_status)},
            {"componentrpc": '#![cfg(feature = "component-rpc")]\n'},
        )

    def test_feature_gated_unit_and_integration_tests_run(self):
        self.classify()
        self.standard(RESULT_OK.format(passed=1, ignored=0))
        self.assertEqual(self.features(), (0, []))
        self.assertEqual((self.root / "union/lib-molten.args").read_text(), "--exact gated_unit\n")
        self.assertEqual((self.root / "union/test-componentrpc.args").read_text(), "--exact rpc_roundtrip\n")

    def test_failing_feature_gated_integration_assertion_denies(self):
        self.classify()
        self.standard("test rpc_roundtrip ... FAILED\ntest result: FAILED. 0 passed; 1 failed; 0 ignored;", 101)
        self.assertEqual(self.features(), (1, ["feature-gated-tests-failed:test:componentrpc"]))

    def test_gated_target_that_executes_nothing_denies_unless_classified(self):
        self.classify()
        self.standard(RESULT_OK.format(passed=0, ignored=1))
        self.assertEqual(self.features(), (1, ["feature-gated-target-executed-no-tests:test:componentrpc"]))
        self.classify('target = "test:componentrpc"\ndisposition = "ignored-inventory"\nreason = "runs on its rail"')
        self.assertEqual(self.features(), (0, []))

    def test_environment_blocked_target_is_recorded_without_running(self):
        self.classify('target = "test:componentrpc"\ndisposition = "environment-blocked"\nreason = "needs a device"')
        self.standard("test result: FAILED. 0 passed; 1 failed; 0 ignored;", 101)
        self.assertEqual(self.features(), (0, []))
        self.assertFalse((self.root / "union/test-componentrpc.args").exists())

    def test_unbuilt_gated_target_and_stale_classification_deny(self):
        self.classify('target = "test:gone"\ndisposition = "ignored-inventory"\nreason = "x"')
        lib = (["default_unit", "gated_unit"], RESULT_OK.format(passed=1, ignored=0), 0)
        self.build({("lib", "molten"): lib}, {"needs_feature": '#![cfg(feature = "x")]\n'})
        self.assertEqual(
            self.features(),
            (1, ["feature-gated-target-not-built:test:needs_feature", "feature-gated-classification-stale:test:gone"]),
        )

    def test_classification_of_a_target_that_runs_tests_is_stale(self):
        self.classify('target = "test:componentrpc"\ndisposition = "ignored-inventory"\nreason = "x"')
        self.standard(RESULT_OK.format(passed=1, ignored=0))
        self.assertEqual(self.features(), (1, ["feature-gated-classification-stale:test:componentrpc"]))


class HegelCoverageTests(Fixture):
    def property(self, attribute: str) -> None:
        write(self.root, "crates/molten-core/tests/p.rs", f"{attribute}\nfn hegel_law(tc: hegel::TestCase) {{}}\n")

    def junit(self, body: str) -> Path:
        path = self.root / "junit.xml"
        path.write_text(f'<testsuites><testsuite name="molten-core::nix-test/0/p">{body}</testsuite></testsuites>')
        return path

    def test_declared_and_executed_property_passes(self):
        self.property("#[hegel::test(test_cases = 4)]")
        junit = self.junit('<testcase name="p::hegel_law"/>')
        self.assertEqual(run("--root", str(self.root), "hegel-coverage", "--junit", str(junit)), (0, []))

    def test_zero_properties_fail_closed(self):
        self.property("#[test]")
        self.assertEqual(
            run("--root", str(self.root), "hegel-coverage"), (1, ["hegel-properties-missing:crates/molten-core"])
        )

    def test_unexecuted_and_failed_properties_deny(self):
        self.property("#[hegel::test]")
        junit = self.junit("")
        self.assertEqual(
            run("--root", str(self.root), "hegel-coverage", "--junit", str(junit)),
            (1, ["hegel-property-not-executed:hegel_law"]),
        )
        junit = self.junit('<testcase name="p::hegel_law"><failure/></testcase>')
        self.assertEqual(
            run("--root", str(self.root), "hegel-coverage", "--junit", str(junit)),
            (1, ["hegel-property-failed:hegel_law"]),
        )


SOURCE_A = "crates/molten-core/src/a.rs"
LINES_A = "fn first() {\n    let a = 1;\n    value.unwrap();\n}\n\nfn second() {\n    let b = 2;\n    value.unwrap();\n}\n"
# Lines FIRST and SECOND are identical `value.unwrap();` statements in different functions.
FIRST, SECOND = 3, 8


class OctetBaselineTests(Fixture):
    SCOPE = "molten,molten-core"

    def setUp(self) -> None:
        super().setUp()
        write(self.root, SOURCE_A, LINES_A)
        write(self.root, "crates/molten-core/src/codec.rs", "pub fn planted(value: Option<u8>) -> u8 { value.unwrap() }\n")
        write(self.root, "src/lib.rs", "fn main() { value.unwrap(); }\n")

    def anchor(self, line: int, file: str = SOURCE_A) -> str:
        return coverage.finding_anchor(self.root, file, line)

    def artifacts(self, findings: list[tuple[str, str, str, int]], command: str = "cargo octet check -p molten -p molten-core") -> Path:
        directory = self.root / "octet"
        directory.mkdir(exist_ok=True)
        index = "".join(
            f"  F{number}  {lint}  {crate}  {file}:{line}\n" for number, (lint, crate, file, line) in enumerate(findings, 1)
        )
        # Real summaries close the index with a `---` rule.
        (directory / "summary.txt").write_text(SUMMARY_HEADER.format(total=len(findings)) + index + "---\n")
        status = {"total_findings": len(findings), "cargo_process_exit": {"classification": "success"}}
        (directory / "status.json").write_text(json.dumps(status))
        (directory / "command.txt").write_text(command + " --artifact-dir out\n")
        return directory

    def check(self, artifacts: Path, *extra: str) -> tuple[int, list[str]]:
        return run("--root", str(self.root), "octet-baseline", "check", "--artifacts", str(artifacts), "--scope", self.SCOPE, *extra)

    def write_baseline(self, artifacts: Path) -> tuple[int, list[str]]:
        return run("--root", str(self.root), "octet-baseline", "write", "--artifacts", str(artifacts), "--scope", self.SCOPE)

    def recorded(self) -> Path:
        artifacts = self.artifacts([("no_unwrap", "molten_core", SOURCE_A, FIRST)] * 2)
        self.assertEqual(self.write_baseline(artifacts), (0, []))
        return artifacts

    def baseline(self) -> dict:
        return json.loads((self.root / "config/testing/octet-workspace-baseline.json").read_text())

    def rewrite_baseline(self, document: dict) -> None:
        (self.root / "config/testing/octet-workspace-baseline.json").write_text(json.dumps(document))

    def test_written_baseline_records_identities_and_matches_its_run(self):
        artifacts = self.recorded()
        document = self.baseline()
        self.assertEqual(document["total"], 2)
        self.assertEqual(document["findings"], [
            {"crate": "molten_core", "file": SOURCE_A, "lint": "no_unwrap", "anchor": self.anchor(FIRST), "count": 2}
        ])
        self.assertEqual(self.check(artifacts), (0, []))

    def test_unrelated_line_shift_keeps_identity(self):
        self.recorded()
        write(self.root, SOURCE_A, "// unrelated\n// header\n// lines\n\n" + LINES_A)
        self.assertEqual(self.check(self.artifacts([("no_unwrap", "molten_core", SOURCE_A, FIRST + 4)] * 2)), (0, []))

    def swap(self, before: int, after: int) -> None:
        artifacts = self.artifacts([("no_unwrap", "molten_core", SOURCE_A, before)])
        self.assertEqual(self.write_baseline(artifacts), (0, []))
        swapped = self.artifacts([("no_unwrap", "molten_core", SOURCE_A, after)])
        key = f"molten_core:{SOURCE_A}:no_unwrap"
        status, diagnostics = self.check(swapped)
        self.assertEqual(
            (status, sorted(diagnostics)),
            (1, [f"octet-baseline-stale:{key}:{self.anchor(before)}:1->0", f"octet-new-finding:{key}:{self.anchor(after)}:0->1"]),
        )

    def test_swapped_finding_on_an_identical_line_elsewhere_denies(self):
        # Finding A on line FIRST is repaired and the same lint appears on the identical line SECOND in
        # another function: the line text and the per-file count are unchanged.
        lines = (self.root / SOURCE_A).read_text().splitlines()
        self.assertEqual(lines[FIRST - 1], lines[SECOND - 1])
        self.swap(FIRST, SECOND)

    def test_swapped_finding_between_identical_windows_denies(self):
        # Every line's two-line context is identical too; only the ordinal tells the findings apart.
        write(self.root, SOURCE_A, "value.unwrap();\n" * 7)
        self.assertEqual(coverage.file_anchors(self.root / SOURCE_A)[2].split("#")[0], self.anchor(5).split("#")[0])
        self.swap(3, 5)

    def test_new_and_reduced_findings_both_deny(self):
        self.recorded()
        key = f"molten_core:{SOURCE_A}:no_unwrap:{self.anchor(FIRST)}"
        more = self.artifacts([("no_unwrap", "molten_core", SOURCE_A, FIRST)] * 3)
        self.assertEqual(self.check(more), (1, [f"octet-baseline-total-mismatch:2!=3", f"octet-new-finding:{key}:2->3"]))
        fewer = self.artifacts([("no_unwrap", "molten_core", SOURCE_A, FIRST)])
        self.assertEqual(self.check(fewer), (1, [f"octet-baseline-total-mismatch:2!=1", f"octet-baseline-stale:{key}:2->1"]))

    def test_duplicate_rows_and_wrong_total_deny(self):
        artifacts = self.recorded()
        document = self.baseline()
        row = dict(document["findings"][0], count=1)
        self.rewrite_baseline(dict(document, findings=[row, row]))
        key = f"molten_core:{SOURCE_A}:no_unwrap:{self.anchor(FIRST)}"
        self.assertEqual(self.check(artifacts), (1, [f"octet-baseline-duplicate-row:{key}"]))
        self.rewrite_baseline(dict(document, total=5))
        self.assertEqual(self.check(artifacts), (1, ["octet-baseline-total-mismatch:5!=2"]))

    def test_old_schema_and_unanchored_findings_deny(self):
        artifacts = self.recorded()
        self.rewrite_baseline(dict(self.baseline(), schema="molten-octet-workspace-baseline-v1"))
        status, diagnostics = self.check(artifacts)
        self.assertEqual((status, diagnostics[0]), (1, f"octet-baseline-schema:molten-octet-workspace-baseline-v1!={coverage.BASELINE_SCHEMA}"))
        beyond = self.artifacts([("no_unwrap", "molten_core", SOURCE_A, 99)])
        self.assertEqual(self.write_baseline(beyond), (1, [f"octet-finding-unanchored:molten_core:{SOURCE_A}:99:no_unwrap"]))

    def test_zero_debt_crate_and_narrowed_scope_deny(self):
        artifacts = self.artifacts([("no_unwrap", "molten", "src/lib.rs", 1)])
        self.assertEqual(self.write_baseline(artifacts), (1, ["octet-zero-debt-crate-has-findings:molten:src/lib.rs:no_unwrap"]))
        self.recorded()
        narrowed = self.artifacts([("no_unwrap", "molten_core", SOURCE_A, FIRST)] * 2, "cargo octet check -p molten")
        self.assertEqual(self.check(narrowed), (1, ["octet-scope-missing-member:molten-core"]))

    def test_crate_filter_compares_only_the_selected_crate(self):
        self.recorded()
        codec = "crates/molten-core/src/codec.rs"
        planted = self.artifacts(
            [("no_unwrap", "molten_core", SOURCE_A, FIRST)] * 2 + [("no_unwrap", "molten_core", codec, 1)],
            "cargo octet check -p molten-core",
        )
        status, diagnostics = run(
            "--root", str(self.root), "octet-baseline", "check", "--artifacts", str(planted),
            "--scope", "molten-core", "--crate", "molten_core",
        )
        self.assertEqual((status, diagnostics), (1, [f"octet-new-finding:molten_core:{codec}:no_unwrap:{self.anchor(1, codec)}:0->1"]))


if __name__ == "__main__":
    unittest.main()
