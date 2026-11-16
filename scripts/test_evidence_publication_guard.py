#!/usr/bin/env python3
"""Positive and negative checks for scripts/evidence-publication-guard.py (no git, no network).

r[verify molten.project.evidence_publication.host_material]
"""
import contextlib
import importlib.util
import io
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

sys.dont_write_bytecode = True
SCRIPT = Path(__file__).with_name("evidence-publication-guard.py")
FIXTURE = Path(__file__).resolve().parent.parent / "tests/fixtures/evidence-publication"
SPEC = importlib.util.spec_from_file_location("evidence_publication_guard", SCRIPT)
guard = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(guard)
B3SUM = shutil.which("b3sum") or "b3sum"


class GuardFixtureTests(unittest.TestCase):
    def setUp(self):
        self.root = Path(tempfile.mkdtemp(prefix="evidence-publication-test-")) / "repo"
        self.addCleanup(shutil.rmtree, self.root.parent)
        # The Nix check reads fixtures from the read-only store; copy bytes, not modes.
        shutil.copytree(FIXTURE, self.root, copy_function=shutil.copyfile)
        for path in [self.root, *self.root.rglob("*")]:
            path.chmod(0o755 if path.is_dir() else 0o644)

    def run_guard(self, *arguments: str) -> tuple[int, str]:
        stderr = io.StringIO()
        with contextlib.redirect_stderr(stderr), contextlib.redirect_stdout(io.StringIO()):
            status = guard.main(["--root", str(self.root), "--b3sum", B3SUM, *arguments])
        return status, stderr.getvalue()

    def findings(self, path: str) -> list[tuple[int, str]]:
        status, stderr = self.run_guard(str(self.root / path))
        prefix = f"{path}:"
        located = []
        for line in stderr.splitlines():
            if line.startswith(prefix):
                number, _column, rule = line[len(prefix):].split(":", 3)[:3]
                located.append((int(number), rule.strip()))
        self.assertEqual(status, 1 if located else 0, stderr)
        return located

    def write(self, path: str, text: str) -> None:
        target = self.root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text)

    def test_new_evidence_with_host_material_is_rejected_with_locations(self):
        self.assertEqual(
            self.findings("evidence/new-run/offending.log"),
            [(2, "home"), (3, "local-host"), (4, "tailnet-ipv4"), (5, "private-ipv4"), (5, "scratch")],
        )

    def test_hosted_runner_home_is_not_personal_but_lookalikes_are(self):
        self.write(
            "evidence/ci/receipt.log",
            "/home/runner/work/aspen\n/home/runners/x\n/home/runner-2/x\n/Users/runner/work\n/home/alice/runner\n",
        )
        self.assertEqual(self.findings("evidence/ci/receipt.log"), [(2, "home"), (3, "home"), (5, "home")])

    def test_ipv4_needs_four_bounded_octets(self):
        self.write(
            "evidence/ci/timing.log",
            "10.094311\n192.168.1\n10.0.0.1.5\nv10.1.2.3\n10.0.0.256\n172.32.0.1\n100.128.0.1\n(10.0.0.1)\n172.31.9.9,\n",
        )
        self.assertEqual(self.findings("evidence/ci/timing.log"), [(8, "private-ipv4"), (9, "private-ipv4")])

    def test_redacted_placeholders_and_public_addresses_pass(self):
        self.assertEqual(self.findings("evidence/new-run/redacted.log"), [])

    def test_grandfathered_file_passes_only_with_its_exact_bytes(self):
        path = "evidence/historical/grandfathered.log"
        self.assertEqual(self.findings(path), [])
        file = self.root / path
        data = bytearray(file.read_bytes())
        data[0] ^= 0x20
        file.write_bytes(bytes(data))
        self.assertEqual(
            self.findings(path), [(1, "home"), (1, "local-host"), (1, "tailnet-ipv4")]
        )

    def test_baseline_does_not_exempt_the_same_bytes_at_another_path(self):
        shutil.copy(self.root / "evidence/historical/grandfathered.log", self.root / "evidence/historical/copy.log")
        self.assertNotEqual(self.findings("evidence/historical/copy.log"), [])

    def test_reviewed_documentation_examples_pass(self):
        self.assertEqual(self.findings("docs/example.md"), [])

    def test_documentation_without_a_review_is_rejected(self):
        self.write("docs/new.md", "Run from `/home/alice/src`; mount /dev/shm/run-1.\n")
        self.assertEqual(self.findings("docs/new.md"), [(1, "home"), (1, "scratch")])

    def test_allowlist_row_covers_only_its_file_and_text(self):
        self.write("docs/other.md", "See `/Users/example` and `/Users/someone`.\n")
        self.assertEqual(self.findings("docs/other.md"), [(1, "home"), (1, "home")])
        self.write("docs/example.md", "A reviewed row names `/Users/example`, not `/Users/someone`.\n")
        self.assertEqual(self.findings("docs/example.md"), [(1, "home")])

    def test_inline_marker_is_not_honoured_in_evidence(self):
        self.write("evidence/new-run/marked.log", "cwd /home/alice evidence-publication: allow\n")
        self.assertEqual(self.findings("evidence/new-run/marked.log"), [(1, "home")])

    def test_cairn_change_and_archive_evidence_are_scanned(self):
        for path in (".cairn/changes/c/evidence/run.log", ".cairn/archive/2026-11-16-c/evidence/run.log"):
            self.write(path, "host 10.1.2.3\n")
            self.assertEqual(self.findings(path), [(1, "private-ipv4")])

    def test_paths_outside_the_scanned_roots_are_ignored(self):
        for path in (".cairn/changes/c/design.md", "tests/run.log"):
            self.write(path, "cwd /home/alice on builder.local\n")
            self.assertEqual(self.findings(path), [])

    def test_new_opaque_container_is_rejected_until_allowlisted_by_digest(self):
        path = "release/bundle.tar.gz"
        (self.root / "release").mkdir()
        (self.root / path).write_bytes(b"\x1f\x8b\x08\x00compressed")
        self.assertEqual(self.findings(path), [(1, "opaque-container")])
        digest = guard.Hasher(B3SUM).digest(self.root / path)
        with (self.root / guard.ALLOWLIST).open("a") as allowlist:
            allowlist.write(f"{path}\topaque-container:{digest}\tfixture: members reviewed\n")
        self.assertEqual(self.findings(path), [])

    def test_whole_tree_rejects_new_files_and_stale_baseline_rows(self):
        status, stderr = self.run_guard("--all")
        self.assertEqual(status, 1)
        self.assertIn("evidence/new-run/offending.log:2:", stderr)
        (self.root / "evidence/new-run/offending.log").unlink()
        self.assertEqual(self.run_guard("--all")[0], 0)
        (self.root / "evidence/historical/grandfathered.log").write_text("redacted on <host>\n")
        status, stderr = self.run_guard("--all")
        self.assertEqual(status, 1)
        self.assertIn("evidence/historical/grandfathered.log: stale baseline entry", stderr)

    def test_malformed_configuration_fails_closed(self):
        baseline = self.root / guard.BASELINE
        baseline.write_text(baseline.read_text() + f"{'0' * 64}\tevidence/a.log\n")
        self.assertEqual(self.run_guard("--all")[0], 2)
        baseline.unlink()
        self.assertEqual(self.run_guard("--all")[0], 2)

    def test_missing_input_fails_closed(self):
        self.assertEqual(self.run_guard(str(self.root / "evidence/absent.log"))[0], 2)


if __name__ == "__main__":
    sys.exit(unittest.main())
