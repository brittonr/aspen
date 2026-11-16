#!/usr/bin/env python3
"""Checks for scripts/readme-quickstart.py using stub executables (no Cargo, no network).

r[verify molten.project.documentation_links.quickstart_commands]
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
SCRIPT = Path(__file__).with_name("readme-quickstart.py")
README = Path(__file__).resolve().parent.parent / "README.md"
SPEC = importlib.util.spec_from_file_location("readme_quickstart", SCRIPT)
quickstart = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(quickstart)

# The Nix build sandbox has no /usr/bin/env, so the stub names the bash on PATH.
STUB = """#!BASH
set -euo pipefail
case "$1 $2" in
  "system-extension run-fixture")
    out=$6
    mkdir -p "$out"
    printf 'status\\n' > "$out/status.preserves"
    echo "system-extension fixture ok profile=$4 manifest=m evidence=1 status=s out=$out"
    ;;
  "system-extension show")
    echo "system-extension status extension=e service=s generation=3 phase=PHASE profile=p health=stopped"
    ;;
  *) exit 64 ;;
esac
"""


class QuickstartTests(unittest.TestCase):
    def setUp(self):
        self.directory = Path(tempfile.mkdtemp(prefix="readme-quickstart-test-"))
        self.addCleanup(shutil.rmtree, self.directory)

    def stub(self, phase: str) -> Path:
        path = self.directory / f"molten-{phase}"
        path.write_text(STUB.replace("BASH", shutil.which("bash") or "/bin/bash", 1).replace("PHASE", phase))
        path.chmod(0o755)
        return path

    def run_quickstart(self, molten: Path, readme: Path = README) -> tuple[int, str]:
        stderr = io.StringIO()
        with contextlib.redirect_stderr(stderr), contextlib.redirect_stdout(io.StringIO()):
            status = quickstart.main(["--molten", str(molten), "--readme", str(readme)])
        return status, stderr.getvalue()

    def test_missing_documented_result_fails(self):
        status, stderr = self.run_quickstart(self.stub("running"))
        self.assertEqual(status, 1)
        self.assertIn("output lacks `phase=stopped`", stderr)

    def test_readme_without_section_is_an_input_error(self):
        readme = self.directory / "README.md"
        readme.write_text("# Aspen\n")
        self.assertEqual(self.run_quickstart(self.stub("stopped"), readme)[0], 2)


if __name__ == "__main__":
    unittest.main(verbosity=2)
