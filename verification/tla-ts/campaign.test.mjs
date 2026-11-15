import assert from "node:assert/strict";
import { test } from "node:test";
import { parseTlc, validatePair } from "./campaign.mjs";

// Synthetic protocol inputs test evidence reading, NOT observed model-run evidence.
const header = "TLC2 Version 2.19 of 08 August 2024 (rev: synthetic-test-input)\n";
const success = header + "Model checking completed. No error has been found.\n" +
  "1,234 states generated, 567 distinct states found, 0 states left on queue.\n";
const baseline = () => ({ tool: "synthetic-checker", status: "success",
  generatedStates: 1234, distinctStates: 567, queuedStates: 0, failure: null });

// r[verify molten.consensus.fast_path_model.evidence]
// r[verify molten.consensus.fast_path_model.validation]
test("complete searches compare generated and distinct state counts", () => {
  const tlc = parseTlc(success, 0);
  assert.deepEqual(validatePair(tlc, baseline()), { verdictsAgree: true, countsAgree: true, exhaustive: true });
  assert.throws(() => validatePair(tlc, { ...baseline(), distinctStates: 566 }), /state counts differ/);
  assert.throws(() => validatePair(tlc, { ...baseline(), generatedStates: 1235 }), /state counts differ/);
});

// r[verify molten.consensus.fast_path_model.nonclaims]
test("parse-only, truncated, timeout and drift outputs never become semantic success", () => {
  assert.throws(() => parseTlc("tla-ts parsed module ConsensusFastPath", 0), /version drift/);
  assert.throws(() => parseTlc(header, 0), /statistics/);
  assert.throws(() => parseTlc(success, null), /termination/);
  assert.throws(() => parseTlc(success.replace("Version 2.19", "Version 2.20"), 0), /version drift/);
  assert.throws(() => validatePair(baseline(), { ...baseline(), status: "error" }), /expected success/);
  assert.throws(() => validatePair(baseline(), { ...baseline(), queuedStates: 1 }), /incomplete/);
  assert.throws(() => validatePair(baseline(), { ...baseline(), distinctStates: undefined }), /missing distinct/);
  assert.throws(() => validatePair(baseline(), { ...baseline(), failure: { message: "unsupported" } }), /evaluation/);
});

// r[verify molten.consensus.fast_path_model.fault_corpus]
test("guard mutant requires the named invariant and causal trace", () => {
  const tlc = parseTlc(header + "Error: Invariant NoMissingPromise is violated.\n" +
    "State 1: <Initial predicate>\nState 2: <FastCommit>\n" +
    "12 states generated, 9 distinct states found, 3 states left on queue.\n", 12);
  const ts = { tool: "synthetic-checker", status: "invariant-violation", generatedStates: 14,
    distinctStates: 10, queuedStates: 4,
    violation: { name: "NoMissingPromise", trace: [{ action: "Init" }, { action: "FastCommit" }] } };
  assert.deepEqual(validatePair(tlc, ts, true), { verdictsAgree: true, countsAgree: false, exhaustive: false });
  assert.throws(() => validatePair(tlc, { ...ts, violation: { name: "TypeOK", trace: [1, 2] } }, true), /wrong mutant/);
  assert.throws(() => validatePair(tlc, { ...ts, violation: { name: "NoMissingPromise", trace: [] } }, true), /causal/);
  assert.throws(() => validatePair(tlc, baseline(), true), /expected invariant-violation/);
});
