#!/usr/bin/env python3
"""Execute the admitted Aspen simulation producer and consume its real receipts.

This fixture never creates source trust, signatures, success wrappers or release
admission. It requires an independently admitted output containing the new CLI.
"""

import argparse
import json
from pathlib import Path
import re
import subprocess


ROOT = Path(__file__).resolve().parent.parent


def command(argv):
    result = subprocess.run(argv, cwd=ROOT, check=True, text=True, capture_output=True)
    return result.stdout


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
    observed = json.loads(command([binary, "evidence", "qualify", "--source-root", str(source_root),
                                   "--scenario", "describe", "--out", "-"]))
    subject = observed["subject"]
    if subject["environment"] != "simulation":
        raise RuntimeError("the fixed native harness producer must retain simulation scope")
    evidence = []
    for scenario, kind in [("granted-send", "positive"), ("denied-send", "negative")]:
        artifact = output / f"{scenario}.preserves"
        summary = command([binary, "evidence", "qualify", "--source-root", str(source_root),
                           "--scenario", scenario, "--out", str(artifact)])
        # This is only a declared reference. The consumer independently computes
        # canonical identity, reauthenticates the selected executable, remeasures
        # compiled facts, validates/replays the native artifact and fixed fixture.
        declared = re.search(r"qualification execution (blake3:[0-9a-f]{64}) ", summary)
        if declared is None:
            raise RuntimeError("producer did not return its declared artifact identity")
        evidence.append({"requirement_id": "molten.testing.capability_context.request_binding",
                         "coverage_kind": kind, "family": "harness-simulation", "scenario": scenario,
                         "subject": subject, "artifact_path": artifact.name, "artifact_ref": declared.group(1)})
    mapping = json.loads((ROOT / "config/capability-report/capabilities.json").read_text())
    capabilities = [{"id": entry["id"], "evidence": evidence if entry["id"] == "harness-runtime-capability-binding" else []}
                    for entry in mapping["capabilities"]]
    report_input = {"schema": "molten.capability-report.v2", "source_root_path": str(source_root),
                    "subject": subject, "capabilities": capabilities}
    input_path = output / "input.json"
    input_path.write_text(json.dumps(report_input, sort_keys=True, indent=2) + "\n")
    base = [args.cargo, "-q", "-Zscript", str(ROOT / "tools/capability-report.rs"),
            "--matrix", str(args.matrix.resolve(strict=True)), "--input", str(input_path)]
    command(base + ["--json-out", str(output / "report.json"), "--markdown-out", str(output / "report.md"),
                    "--canonical-out", str(output / "report.bin")])
    rows = {row["definition"]["id"]: row for row in json.loads((output / "report.json").read_text())["capabilities"]}
    harness = rows["harness-runtime-capability-binding"]["execution"]
    if harness["status"] != "bounded-observed" or harness["missing"]:
        raise RuntimeError("actual signed-source positive/negative simulation was not qualified")
    if any(row["execution"]["status"] == "bounded-observed" for key, row in rows.items()
           if key != "harness-runtime-capability-binding"):
        raise RuntimeError("harness evidence was promoted to an unexecuted product capability")
    command(base + ["--canonical-out", str(output / "report-repeat.bin")])
    if (output / "report.bin").read_bytes() != (output / "report-repeat.bin").read_bytes():
        raise RuntimeError("identical retained inputs produced different canonical reports")
    print(f"actual bounded simulation qualification retained at {output}; no distributed/live/release claim")


if __name__ == "__main__":
    main()
