import { readFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

// This adapter executes the published model checker; parsing alone is never a verdict.
const [spec, config, root] = process.argv.slice(2);
try {
  if (!spec || !config || !root) throw new Error("usage: check.mjs SPEC CFG EXTRACTED_NPM_ROOT");
  const metadata = JSON.parse(readFileSync(join(root, "package/package.json"), "utf8"));
  if (metadata.name !== "tla-ts" || metadata.version !== "0.0.10") {
    throw new Error("TLA-ts publication drift: expected tla-ts@0.0.10");
  }
  const { check, formatValue } = await import(pathToFileURL(join(root, "package/dist/index.js")).href);
  if (typeof check !== "function") throw new Error("pinned TLA-ts does not export check(CheckOptions)");
  const result = await check({
    source: readFileSync(spec, "utf8"),
    config: readFileSync(config, "utf8"),
    version: "1.7.4",
    workers: 1,
    livenessCheck: "final",
    onPrint: (text) => process.stderr.write(`${text}\n`),
  });
  const trace = (result.violation?.trace ?? []).map((step) => ({
    action: step.action ?? null,
    state: Object.fromEntries(Object.entries(step.state).map(([key, value]) => [key, formatValue(value)])),
  }));
  console.log(JSON.stringify({
    tool: "tla-ts@0.0.10",
    status: result.status,
    generatedStates: result.generatedStates,
    distinctStates: result.distinctStates,
    queuedStates: result.queuedStates,
    depth: result.depth,
    violation: result.violation ? { name: result.violation.name, trace, loopStart: result.violation.loopStart } : null,
    failure: result.failure ? { code: result.failure.code, message: result.failure.message } : null,
  }));
  if (result.status !== "success" && result.status !== "invariant-violation") process.exitCode = 1;
} catch (error) {
  console.log(JSON.stringify({ tool: "tla-ts@0.0.10", status: "error", error: `${error.name}: ${error.message}` }));
  process.exitCode = 1;
}
