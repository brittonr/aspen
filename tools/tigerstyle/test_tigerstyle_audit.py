#!/usr/bin/env python3
import importlib.util
import json
import pathlib
import sys
import unittest


REPO_ROOT = pathlib.Path(__file__).resolve().parents[2]
SCRIPT_PATH = REPO_ROOT / "scripts" / "tigerstyle-audit.py"
FIXTURE_PATH = REPO_ROOT / "tools" / "tigerstyle" / "fixtures" / "trait_declaration_false_positive.rs"
KEY_MANAGER_PATH = REPO_ROOT / "crates" / "aspen-trust" / "src" / "key_manager.rs"
FORGE_VERIFIED_PATH = REPO_ROOT / "crates" / "aspen-forge" / "src" / "verified" / "mod.rs"


spec = importlib.util.spec_from_file_location("tigerstyle_audit", SCRIPT_PATH)
module = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = module
assert spec.loader is not None
spec.loader.exec_module(module)


class TigerStyleAuditRegressionTest(unittest.TestCase):
    def test_trait_declaration_does_not_count_as_body(self) -> None:
        functions = module.find_functions(FIXTURE_PATH)
        names = [function.function for function in functions]
        self.assertEqual(names, ["declaration_only", "regular_function"])

        declaration_impl = functions[0]
        self.assertEqual(declaration_impl.line, 18)
        self.assertEqual(declaration_impl.end_line, 30)
        self.assertEqual(declaration_impl.line_count, 13)

    def test_signature_parser_stops_at_top_level_semicolon(self) -> None:
        lines = FIXTURE_PATH.read_text(encoding="utf-8").splitlines(keepends=True)
        match = module.FUNCTION_START_RE.match(lines[1])
        self.assertIsNotNone(match)
        has_body, brace_line, _brace_col, _signature = module.signature_result(lines, 1, match.start())
        self.assertFalse(has_body)
        self.assertEqual(brace_line + 1, 12)

    def test_json_inventory_reports_impl_only(self) -> None:
        result = module.scan([FIXTURE_PATH], line_limit=5)
        inventory = json.loads(json.dumps(result))
        functions = {(item["function"], item["rule"]) for item in inventory["hotspots"]}
        self.assertIn(("declaration_only", "function_length"), functions)
        self.assertIn(("regular_function", "usize_api"), functions)
        self.assertNotIn((None, "function_length"), functions)

    def test_key_manager_file_no_longer_reports_parse_error(self) -> None:
        result = module.scan([KEY_MANAGER_PATH], line_limit=70)
        self.assertEqual(result["parse_errors"], [])

    def test_verified_time_hotspots_ignore_doc_comments(self) -> None:
        hotspots = module.verified_time_hotspots([FORGE_VERIFIED_PATH])
        self.assertEqual(hotspots, [])


if __name__ == "__main__":
    unittest.main()
