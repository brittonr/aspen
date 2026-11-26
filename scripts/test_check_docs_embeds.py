#!/usr/bin/env python3
"""Positive and negative probes for scripts/check-docs-embeds.py (no git, no network).

r[verify molten.project.real_module_encapsulation]
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
SCRIPT = Path(__file__).with_name("check-docs-embeds.py")
SPEC = importlib.util.spec_from_file_location("check_docs_embeds", SCRIPT)
guard = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(guard)


class DocsEmbedGuardTests(unittest.TestCase):
    def setUp(self):
        self.root = Path(tempfile.mkdtemp(prefix="docs-embed-guard-test-"))
        self.addCleanup(shutil.rmtree, self.root)

    def write(self, path: str, text: str) -> None:
        target = self.root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text)

    def run_guard(self) -> tuple[int, str]:
        stderr = io.StringIO()
        with contextlib.redirect_stderr(stderr):
            status = guard.main(["--root", str(self.root)])
        return status, stderr.getvalue()

    def test_concat_manifest_dir_docs_probe_is_rejected_with_location(self):
        self.write(
            "src/wasm/profile.rs",
            "const OK: &str = include_str!(\"../config/a.json\");\n"
            "const PROFILE: &str =\n"
            "    include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/docs/wasm/generated/profile.json\"));\n",
        )
        status, stderr = self.run_guard()
        self.assertEqual(status, 1)
        self.assertIn("src/wasm/profile.rs:3:", stderr)
        self.assertIn("/docs/wasm/generated/profile.json", stderr)

    def test_multiline_relative_and_bytes_embeds_are_rejected(self):
        self.write(
            "crates/kernel/src/lib.rs",
            "const A: &[u8] = include_bytes!(\n    \"../../docs/fixture.bin\"\n);\n"
            "const B: &str = include_str!(\"docs/notes.md\");\n",
        )
        status, stderr = self.run_guard()
        self.assertEqual(status, 1)
        self.assertEqual(stderr.count("embeds documentation path"), 2)

    def test_config_embeds_lookalike_names_and_vendor_tree_are_accepted(self):
        self.write(
            "src/ok.rs",
            "const A: &str = include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/config/x/generated/p.json\"));\n"
            "const B: &str = include_str!(\"../mydocs/readme.md\");\n"
            "// a docs/ path outside any include is prose\n"
            "const C: &str = \"docs/not-embedded.md\";\n",
        )
        self.write("vendor/dep/src/lib.rs", "const D: &str = include_str!(\"../docs/x.md\");\n")
        self.write("src/target/generated.rs", "const E: &str = include_str!(\"/docs/y.md\");\n")
        self.assertEqual(self.run_guard(), (0, ""))

    def test_unreadable_root_fails_closed(self):
        stderr = io.StringIO()
        with contextlib.redirect_stderr(stderr):
            status = guard.main(["--root", str(self.root / "missing")])
        self.assertEqual(status, 2)


if __name__ == "__main__":
    unittest.main()
