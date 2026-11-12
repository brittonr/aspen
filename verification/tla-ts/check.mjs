import { readFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

const [spec, root] = process.argv.slice(2);
const { parseModule } = await import(pathToFileURL(join(root, "package/dist/index.js")).href);
const module = parseModule(readFileSync(spec, "utf8"));
if (!module || module.name !== "ConsensusFastPath") {
  throw new Error("TLA-ts parsed an unexpected module");
}
console.log(`tla-ts parsed module ${module.name}`);
