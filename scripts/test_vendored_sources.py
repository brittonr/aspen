#!/usr/bin/env python3
"""Positive and negative checks for scripts/vendored-sources.py (no network, no nickel)."""
import importlib.util
import os
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("vendored-sources.py")
SPEC = importlib.util.spec_from_file_location("vendored_sources", SCRIPT)
vendored = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(vendored)
B3SUM = shutil.which("b3sum") or "b3sum"
REVISION = "0" * 40


class CheckTreeTests(unittest.TestCase):
    def setUp(self):
        self.root = Path(tempfile.mkdtemp(prefix="vendored-sources-test-"))
        self.addCleanup(shutil.rmtree, self.root)
        destination = self.root / "vendor/upstream"
        (destination / "crates/core/src").mkdir(parents=True)
        (destination / "LICENSE").write_text("license\n")
        (destination / "crates/core/src/lib.rs").write_text("pub fn f() {}\n")
        (destination / "crates/core/Cargo.toml.orig").write_text("[package]\nversion.workspace = true\n")
        (destination / "crates/core/Cargo.toml").write_text("[package]\nversion = \"0.1.0\"\n")
        fixture = self.root / "tests/fixture.json"
        fixture.parent.mkdir(parents=True)
        fixture.write_text("{}\n")
        (self.root / "NOTICES.md").write_text(
            f"vendor/upstream at {REVISION}\ntests/fixture.json at {REVISION}\n"
        )
        self.policy = {
            "notices": "NOTICES.md",
            "sources": [{
                "name": "upstream",
                "repository": "rad://example",
                "revision": REVISION,
                "destination": "vendor/upstream",
                "selected_paths": ["LICENSE", "crates/core"],
                "renamed_originals": [
                    {"upstream": "crates/core/Cargo.toml", "destination": "crates/core/Cargo.toml.orig"}
                ],
                "aspen_owned": [{"path": "crates/core/Cargo.toml", "derived_from": "crates/core/Cargo.toml.orig",
                                 "change": "normalized"}],
            }],
            "fixture_bindings": [{"repository": "rad://producer", "revision": REVISION,
                                  "upstream_path": "fixtures/fixture.json", "local_path": "tests/fixture.json"}],
        }
        files = [
            self.record("LICENSE", "LICENSE", "upstream"),
            self.record("crates/core/src/lib.rs", "crates/core/src/lib.rs", "upstream"),
            self.record("crates/core/Cargo.toml.orig", "crates/core/Cargo.toml", "renamed-upstream"),
            self.record("crates/core/Cargo.toml", None, "aspen-owned"),
        ]
        binding = {**self.policy["fixture_bindings"][0], **vendored.file_record(fixture, B3SUM)}
        self.lock = {
            "schema": vendored.LOCK_SCHEMA,
            "sources": [{"name": "upstream", "repository": "rad://example", "revision": REVISION,
                         "destination": "vendor/upstream", "files": files}],
            "fixture_bindings": [binding],
        }

    def record(self, path, upstream_path, role):
        on_disk = self.root / "vendor/upstream" / path
        return {"path": path, "upstream_path": upstream_path, "role": role, "executable": False,
                **vendored.file_record(on_disk, B3SUM)}

    def errors(self):
        return vendored.check_tree(self.root, self.policy, self.lock, B3SUM)

    def assert_rejected(self, fragment):
        errors = self.errors()
        self.assertTrue(any(fragment in error for error in errors), errors)

    def test_exact_tree_passes(self):
        self.assertEqual(self.errors(), [])

    def test_extra_file_is_rejected(self):
        (self.root / "vendor/upstream/crates/core/src/extra.rs").write_text("")
        self.assert_rejected("unexpected file crates/core/src/extra.rs")

    def test_missing_file_is_rejected(self):
        (self.root / "vendor/upstream/LICENSE").unlink()
        self.assert_rejected("missing file LICENSE")

    def test_changed_byte_is_rejected(self):
        (self.root / "vendor/upstream/crates/core/src/lib.rs").write_text("pub fn g() {}\n")
        self.assert_rejected("crates/core/src/lib.rs git_blob")

    def test_symlink_is_rejected(self):
        os.symlink("LICENSE", self.root / "vendor/upstream/link")
        self.assert_rejected("link is a symlink")

    def test_unselected_upstream_path_is_rejected(self):
        self.policy["sources"][0]["selected_paths"] = ["crates/core"]
        self.assert_rejected("LICENSE is not an unchanged selected upstream path")

    def test_undeclared_rename_is_rejected(self):
        self.policy["sources"][0]["renamed_originals"] = []
        self.assert_rejected("is not a declared rename")

    def test_undeclared_aspen_owned_file_is_rejected(self):
        self.policy["sources"][0]["aspen_owned"] = []
        self.assert_rejected("is not a declared Aspen-owned file")

    def test_revision_drift_is_rejected(self):
        self.lock["sources"][0]["revision"] = "1" * 40
        self.assert_rejected("lock revision")

    def test_notice_without_selected_revision_is_rejected(self):
        (self.root / "NOTICES.md").write_text(f"vendor/upstream at {'1' * 40}\ntests/fixture.json at {REVISION}\n")
        self.policy["sources"][0]["revision"] = "2" * 40
        self.lock["sources"][0]["revision"] = "2" * 40
        self.assert_rejected("lacks a notice naming vendor/upstream at revision " + "2" * 40)

    def test_changed_fixture_is_rejected(self):
        (self.root / "tests/fixture.json").write_text("{\"changed\": true}\n")
        self.assert_rejected("fixture tests/fixture.json: git_blob")

    def test_unknown_lock_schema_is_rejected(self):
        self.lock["schema"] = "other"
        self.assert_rejected("has schema")


if __name__ == "__main__":
    sys.exit(unittest.main())
