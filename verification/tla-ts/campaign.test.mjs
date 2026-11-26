import assert from "node:assert/strict";
import { test } from "node:test";
import { MODEL_RUNS, parseTlc, requireCheckerCapture, validatePair } from "./campaign.mjs";
import { DIVE_RELEASE, parseDive, parseDiveVersion, readDiveReport, verifyDiveArchive } from "./dive.mjs";

// Synthetic protocol inputs test evidence reading, NOT observed model-run evidence.
const header = "TLC2 Version 2.19 of 08 August 2024 (rev: synthetic-test-input)\n";
const success = header + "Model checking completed. No error has been found.\n" +
  "1,234 states generated, 567 distinct states found, 0 states left on queue.\n";
const baseline = () => ({ tool: "synthetic-checker", status: "success",
  generatedStates: 1234, distinctStates: 567, queuedStates: 0, failure: null });

const diveSuccess = () => ({
  schema_version: 3, command: "check", check_kind: "temporal", verdict: "success", exit_code: 0,
  options: { semantic_profile: "tlc", exit_codes: "tlc", workers: { requested: 1, used: 1 } },
  execution: { stage: "complete", capture_completed: true, exhausted: true },
  statistics: { generated: 1234, distinct: 567, left_on_queue: 0 }, property: null, counterexample: null,
});

function diveViolation(property = "CommittedOrderAgreement") {
  const { execution, ...common } = diveSuccess();
  return { ...common,
    check_kind: "safety", verdict: "invariant-violation", exit_code: 12, property,
    statistics: { generated: 13, distinct: 10, left_on_queue: 2 },
    counterexample: { ending: "finite", length: 2, states: [
      { index: 1, label: "<Initial predicate>", action: null, values: { committed: false } },
      { index: 2, label: "<FastCommit>", action: "FastCommit", values: { committed: true } },
    ] },
  };
}

// r[verify molten.consensus.fast_path_model.evidence]
// r[verify molten.consensus.fast_path_model.validation]
test("complete searches compare generated and distinct state counts", () => {
  const tlc = parseTlc(success, 0);
  const dive = parseDive(diveSuccess(), 0);
  assert.deepEqual(validatePair(tlc, baseline(), dive), { verdictsAgree: true, countsAgree: true, exhaustive: true });
  assert.throws(() => validatePair(tlc, { ...baseline(), distinctStates: 566 }, dive), /state counts differ/);
  assert.throws(() => validatePair(tlc, { ...baseline(), generatedStates: 1235 }, dive), /state counts differ/);
  for (const statistics of [{ generated: 1234, distinct: 566 }, { generated: 1235, distinct: 567 }]) {
    const divergent = parseDive({ ...diveSuccess(), statistics: { ...statistics, left_on_queue: 0 } }, 0);
    assert.throws(() => validatePair(tlc, baseline(), divergent), /state counts differ: TLC .* vs dive 0\.1\.0/);
  }
});

// r[verify molten.consensus.fast_path_model.nonclaims]
test("parse-only, truncated, timeout and drift outputs never become semantic success", () => {
  assert.throws(() => parseTlc("tla-ts parsed module ConsensusFastPath", 0), /version drift/);
  assert.throws(() => parseTlc(header, 0), /statistics/);
  assert.throws(() => parseTlc(success, null), /termination/);
  assert.throws(() => parseTlc(success.replace("Version 2.19", "Version 2.20"), 0), /version drift/);
  assert.throws(() => validatePair(baseline(), { ...baseline(), status: "error" }, baseline()), /expected success/);
  assert.throws(() => validatePair(baseline(), { ...baseline(), queuedStates: 1 }, baseline()), /incomplete/);
  assert.throws(() => validatePair(baseline(), { ...baseline(), distinctStates: undefined }, baseline()), /missing distinct/);
  assert.throws(() => validatePair(baseline(), { ...baseline(), failure: { message: "unsupported" } }, baseline()), /evaluation/);
});

// r[verify molten.consensus.fast_path_model.fault_corpus]
test("each mutant requires its named invariant and causal trace", () => {
  const expected = "CommittedOrderAgreement";
  const tlc = parseTlc(header + `Error: Invariant ${expected} is violated.\n` +
    "State 1: <Initial predicate>\nState 2: <FastCommit>\n" +
    "12 states generated, 9 distinct states found, 3 states left on queue.\n", 12);
  const ts = { tool: "synthetic-checker", status: "invariant-violation", generatedStates: 14,
    distinctStates: 10, queuedStates: 4,
    violation: { name: expected, trace: [{ action: "Init" }, { action: "FastCommit" }] } };
  const dive = parseDive(diveViolation(), 12, true);
  assert.deepEqual(validatePair(tlc, ts, dive, expected), { verdictsAgree: true, countsAgree: false, exhaustive: false });
  assert.throws(() => validatePair(tlc, { ...ts, violation: { name: "TypeOK", trace: [1, 2] } }, dive, expected), /wrong mutant/);
  assert.throws(() => validatePair(tlc, ts, dive, "Recoverability"), /wrong mutant/);
  assert.throws(() => validatePair(tlc, { ...ts, violation: { name: expected, trace: [] } }, dive, expected), /causal/);
  assert.throws(() => validatePair(tlc, baseline(), dive, expected), /expected invariant-violation/);
  assert.throws(() => validatePair(tlc, ts, parseDive(diveViolation("TypeOK"), 12, true), expected), /dive .*wrong mutant/);
  // A mutant whose verdict is a clean pass is never treated as caught.
  assert.throws(() => validatePair(baseline(), baseline(), parseDive(diveSuccess(), 0), expected), /expected invariant-violation/);
});

test("every configured mutant names a semantic safety invariant", () => {
  const semantic = new Set(["CommittedOrderAgreement", "Recoverability", "ExecutionOrderAgreement",
    "LinearizableConflictOrder", "AtMostOnceApplication"]);
  assert.ok(MODEL_RUNS.some((run) => run.violation === null && run.bounds.replicas === 3));
  for (const run of MODEL_RUNS.filter((run) => run.violation !== null)) assert.ok(semantic.has(run.violation), run.label);
  // Recovery-quorum intersection is only load-bearing with more replicas than
  // the superquorum: a four-replica baseline and its small-quorum mutant must
  // share bounds.
  const four = MODEL_RUNS.filter((run) => run.bounds.replicas === 4);
  assert.deepEqual(four.map((run) => run.violation), [null, "CommittedOrderAgreement"]);
  assert.deepEqual(four[0].bounds, four[1].bounds);
  assert.ok(MODEL_RUNS.some((run) => run.label === "insufficient-quorum"));
  assert.equal(new Set(MODEL_RUNS.map((run) => run.config)).size, MODEL_RUNS.length);
});

// r[verify molten.consensus.fast_path_model.dive_parity]
test("Dive refusal and budget stop name the cause even if counts are present", () => {
  const refused = { ...diveSuccess(), verdict: "unsupported", exit_code: 3, message: "VIEW with liveness" };
  assert.throws(() => parseDive(refused, 3), /refused the check \(exit 3\): VIEW with liveness/);
  assert.throws(() => readDiveReport(Buffer.alloc(0), 3, "unsupported: VIEW with liveness\n"),
    /refused the check \(exit 3\): .*VIEW with liveness/);
  const stopped = { ...diveSuccess(), verdict: "stopped", exit_code: 5, message: "stopped during graph capture", stopped: {} };
  assert.throws(() => parseDive(stopped, 5), /budget stop \(exit 5\): stopped during graph capture/);
});

test("Dive pin, version, timeout and malformed or incomplete reports fail closed", () => {
  assert.throws(() => verifyDiveArchive(Buffer.from("wrong download")), /archive pin mismatch/);
  assert.equal(parseDiveVersion(`${DIVE_RELEASE.version}\n`, 0), DIVE_RELEASE.version);
  assert.throws(() => parseDiveVersion(DIVE_RELEASE.version.replace("94cbbed9", "00000000"), 0), /version drift/);
  assert.throws(() => parseDiveVersion(DIVE_RELEASE.version, 1), /version probe failed/);
  assert.throws(() => requireCheckerCapture({ error: { code: "ETIMEDOUT" }, signal: "SIGKILL" },
    "baseline-dive", 180_000, 180_000), /baseline-dive: checker timeout/);
  assert.throws(() => requireCheckerCapture({ error: { code: "ENOBUFS", message: "buffer overrun" } },
    "baseline-dive", 1, 180_000), /checker process failed: buffer overrun/);
  assert.throws(() => readDiveReport(Buffer.from('{"schema_version":3,'), 0), /malformed or incomplete JSON/);
  assert.throws(() => parseDive({ ...diveSuccess(), schema_version: 4 }, 0), /unsupported report schema/);
  assert.throws(() => parseDive({ ...diveSuccess(), command: "parse" }, 0), /unsupported report schema or command/);
  assert.throws(() => parseDive(diveSuccess(), 3), /exit code mismatch/);
  assert.throws(() => parseDive({ ...diveSuccess(), options: {} }, 0), /single-worker TLC compatibility/);
  assert.throws(() => parseDive({ ...diveSuccess(), statistics: { distinct: 567, left_on_queue: 0 } }, 0), /invalid.*counts/);
  assert.throws(() => parseDive({ ...diveSuccess(), check_kind: "safety" }, 0), /incomplete temporal search/);
  assert.throws(() => parseDive({ ...diveSuccess(), execution: { stage: "complete", capture_completed: false, exhausted: true } }, 0), /incomplete temporal search/);
  const report = diveViolation();
  report.counterexample.states.pop();
  assert.throws(() => parseDive(report, 12, true), /incomplete causal counterexample/);
});
