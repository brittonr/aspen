#!/usr/bin/env python3
"""Positive and negative checks for scripts/check-doc-links.py (no git, no network).

r[verify molten.project.documentation_links.local_targets]
"""
import contextlib
import importlib.util
import io
import sys
import unittest
from pathlib import Path

sys.dont_write_bytecode = True
SCRIPT = Path(__file__).with_name("check-doc-links.py")
FIXTURE = Path(__file__).resolve().parent.parent / "tests/fixtures/doc-links"
SPEC = importlib.util.spec_from_file_location("check_doc_links", SCRIPT)
links = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(links)


class DocLinkFixtureTests(unittest.TestCase):
    def run_checker(self, *files: str) -> tuple[int, list[str]]:
        stderr = io.StringIO()
        with contextlib.redirect_stderr(stderr), contextlib.redirect_stdout(io.StringIO()):
            status = links.main(["--root", str(FIXTURE), *files])
        return status, stderr.getvalue().splitlines()

    def test_valid_links_anchors_and_skipped_targets_pass(self):
        self.assertEqual(self.run_checker("README.md"), (0, []))

    def test_broken_targets_and_anchors_are_reported_with_locations(self):
        status, findings = self.run_checker("docs/broken.md")
        self.assertEqual(status, 1)
        self.assertEqual(
            findings,
            [
                "docs/broken.md:3: missing.md: target does not exist",
                "docs/broken.md:4: guide.md#no-such-anchor: anchor #no-such-anchor is not defined",
                "docs/broken.md:5: data.txt#L99: line anchor #L99 exceeds the file's line count",
                "docs/broken.md:6: ../../outside.md: target leaves the repository",
            ],
        )

    def test_default_scope_is_entry_pages_and_docs_markdown(self):
        names = [path.relative_to(FIXTURE).as_posix() for path in links.default_files(FIXTURE)]
        self.assertEqual(names, ["README.md", "docs/broken.md", "docs/guide.md"])
        status, findings = self.run_checker()
        self.assertEqual(status, 1)
        self.assertEqual(len(findings), 4)

    def test_github_heading_slugs(self):
        self.assertEqual(links.github_slug("`molten` CLI: quick start"), "molten-cli-quick-start")
        self.assertEqual(links.github_slug("Before an [external](x.md) release"), "before-an-external-release")
        self.assertEqual(links.github_slug("Prolly / Redb & storage"), "prolly--redb--storage")

    def test_missing_input_file_fails_closed(self):
        self.assertEqual(self.run_checker("docs/absent.md")[0], 2)


if __name__ == "__main__":
    unittest.main(verbosity=2)
