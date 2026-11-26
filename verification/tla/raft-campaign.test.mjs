import assert from "node:assert/strict";
import { test } from "node:test";
import { parseTlc } from "../tla-ts/campaign.mjs";
import { RUNS, validateRun } from "./raft-campaign.mjs";

// Synthetic TLC transcripts test evidence reading, NOT observed model-run evidence.
const header = "TLC2 Version 2.19 of 08 August 2024 (rev: synthetic-test-input)\n";
const success = parseTlc(header + "Model checking completed. No error has been found.\n" +
  "1,234 states generated, 567 distinct states found, 0 states left on queue.\n", 0);
const violation = (name, states = 2) => parseTlc(header + `Error: Invariant ${name} is violated.\n` +
  Array.from({ length: states }, (_, index) => `State ${index + 1}: <Action>\n`).join("") +
  "12 states generated, 9 distinct states found, 3 states left on queue.\n", 12);

// r[verify molten.consensus.production_raft_formal_model]
test("every run is a baseline or a mutant with a named Raft safety invariant", () => {
  const invariants = new Set(["ElectionSafety", "LogMatching", "LeaderCompleteness", "StateMachineSafety",
    "ReadIndexLinearizable"]);
  assert.ok(RUNS.some((run) => run.expected === null));
  assert.ok(RUNS.filter((run) => run.expected !== null).length >= 3);
  for (const run of RUNS) assert.ok(run.expected === null || invariants.has(run.expected), run.label);
  assert.equal(new Set(RUNS.map((run) => run.config)).size, RUNS.length);
});

test("baselines must exhaust the graph without a violation", () => {
  assert.deepEqual(validateRun(success, null), { exhaustive: true });
  assert.throws(() => validateRun({ ...success, queuedStates: 1 }, null), /incomplete/);
  assert.throws(() => validateRun(violation("LeaderCompleteness"), null), /expected success/);
  assert.throws(() => validateRun({ ...success, distinctStates: 0 }, null), /distinct-state/);
});

test("mutants must fail with the named invariant and a causal trace", () => {
  assert.deepEqual(validateRun(violation("LeaderCompleteness"), "LeaderCompleteness"), { exhaustive: false });
  assert.throws(() => validateRun(success, "LeaderCompleteness"), /expected invariant-violation/);
  assert.throws(() => validateRun(violation("TypeOK"), "LeaderCompleteness"), /expected LeaderCompleteness, got TypeOK/);
  assert.throws(() => validateRun(violation("ElectionSafety", 1), "ElectionSafety"), /causal/);
});
