#!/usr/bin/env python3
"""Execute the admitted Aspen native producers and consume their real receipts.

For every reviewed capability with a native producer (the harness simulation
and the locally runnable product capabilities), the admitted binary executes
each fixed positive/negative scenario named by the checked-in evidence matrix,
and the report tool independently revalidates every retained receipt against
the selected executable's own re-execution. Each capability is qualified under
its own exact subject (profile + environment); evidence never crosses into
another capability or a wider environment (VM, cross-host, release).

This fixture never creates source trust, signatures, success wrappers or release
admission. It requires an independently admitted output containing the CLI.
"""

import argparse
import json
from pathlib import Path
import re
import subprocess


ROOT = Path(__file__).resolve().parent.parent
SCENARIO = re.compile(r" evidence qualify .*--scenario ([a-z0-9-]+) ")


def command(argv):
    result = subprocess.run(argv, cwd=ROOT, check=True, text=True, capture_output=True)
    return result.stdout


def scenarios_for(capability, matrix):
    """Exact (requirement, coverage kind, scenario) rows of the reviewed matrix."""
    rows = []
    for entry in matrix["entries"]:
        found = SCENARIO.search(entry["command"])
        if found and entry["requirement_id"] in capability["requirements"]:
            rows.append((entry["requirement_id"], entry["coverage_kind"], found.group(1)))
    return rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path, required=True)
    parser.add_argument("--matrix", type=Path, required=True, help="existing Nickel matrix exported to JSON")
    parser.add_argument("--out", type=Path, required=True, help="new exclusively owned retained run directory")
    parser.add_argument("--cargo", default="cargo")
    args = parser.parse_args()
    source_root = args.source_root.resolve(strict=True)
    trust = json.loads((source_root / "source-gate-trust.json").read_text())
    binary = str(Path(trust["output"]) / "bin/molten")
    output = args.out.absolute()
    output.mkdir(parents=True, exist_ok=False)
    matrix_path = args.matrix.resolve(strict=True)
    matrix = json.loads(matrix_path.read_text())
    mapping = json.loads((ROOT / "config/capability-report/capabilities.json").read_text())
    observed = json.loads(command([binary, "evidence", "qualify", "--source-root", str(source_root),
                                   "--scenario", "describe", "--out", "-"]))
    qualified = []
    for capability in mapping["capabilities"]:
        subject = observed["subjects"].get(capability["id"])
        if subject is None:
            continue
        families = [family for family in capability["families"] if family["environment"] == subject["environment"]]
        if len(families) != 1:
            raise RuntimeError(f"{capability['id']}: native subject environment is not one reviewed family")
        family = families[0]["id"]
        rows = scenarios_for(capability, matrix)
        if not rows:
            raise RuntimeError(f"{capability['id']}: no matrix row names a native producer scenario")
        run = output / capability["id"]
        run.mkdir()
        evidence = []
        for requirement_id, kind, scenario in rows:
            artifact = run / f"{scenario}.preserves"
            summary = command([binary, "evidence", "qualify", "--source-root", str(source_root),
                               "--scenario", scenario, "--out", str(artifact)])
            # This is only a declared reference. The consumer independently computes
            # canonical identity, reauthenticates the selected executable, remeasures
            # compiled facts, and requires equality with the executable's own re-run.
            declared = re.search(r"qualification execution (blake3:[0-9a-f]{64}) ", summary)
            if declared is None:
                raise RuntimeError("producer did not return its declared artifact identity")
            evidence.append({"requirement_id": requirement_id, "coverage_kind": kind, "family": family,
                             "scenario": scenario, "subject": subject,
                             "artifact_path": artifact.name, "artifact_ref": declared.group(1)})
        capabilities = [{"id": entry["id"], "evidence": evidence if entry["id"] == capability["id"] else []}
                        for entry in mapping["capabilities"]]
        report_input = {"schema": "molten.capability-report.v2", "source_root_path": str(source_root),
                        "subject": subject, "capabilities": capabilities}
        input_path = run / "input.json"
        input_path.write_text(json.dumps(report_input, sort_keys=True, indent=2) + "\n")
        base = [args.cargo, "-q", "-Zscript", str(ROOT / "tools/capability-report.rs"),
                "--matrix", str(matrix_path), "--input", str(input_path)]
        command(base + ["--json-out", str(run / "report.json"), "--markdown-out", str(run / "report.md"),
                        "--canonical-out", str(run / "report.bin")])
        report_rows = {row["definition"]["id"]: row for row in json.loads((run / "report.json").read_text())["capabilities"]}
        execution = report_rows[capability["id"]]["execution"]
        if execution["status"] != "bounded-observed" or execution["missing"]:
            raise RuntimeError(f"{capability['id']}: actual positive/negative native execution was not qualified")
        if any(row["execution"]["status"] == "bounded-observed" for key, row in report_rows.items()
               if key != capability["id"]):
            raise RuntimeError(f"{capability['id']}: evidence was promoted to another capability")
        command(base + ["--canonical-out", str(run / "report-repeat.bin")])
        if (run / "report.bin").read_bytes() != (run / "report-repeat.bin").read_bytes():
            raise RuntimeError("identical retained inputs produced different canonical reports")
        qualified.append(f"{capability['id']} ({subject['environment']})")
    print(f"bounded native qualification retained at {output}: {', '.join(qualified)}; "
          "no VM, cross-host, production or release claim")


if __name__ == "__main__":
    main()
