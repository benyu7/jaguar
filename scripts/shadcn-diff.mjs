import { readdir } from "node:fs/promises";
import { spawn } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const uiDir = path.join(root, "src", "components", "ui");

const names = (await readdir(uiDir))
  .filter((file) => file.endsWith(".tsx"))
  .map((file) => path.basename(file, ".tsx"))
  .sort();

if (names.length === 0) {
  console.error(`No components in ${path.relative(root, uiDir)}`);
  process.exit(1);
}

function diffComponent(name) {
  return new Promise((resolve, reject) => {
    const child = spawn(
      "npx",
      ["--yes", "shadcn@latest", "add", name, "--diff"],
      { cwd: root },
    );
    let output = "";
    child.stdout.on("data", (chunk) => {
      output += chunk;
    });
    child.stderr.on("data", (chunk) => {
      output += chunk;
    });
    child.on("error", reject);
    child.on("close", (code) => resolve({ code, output }));
  });
}

function plain(text) {
  return text.replaceAll(/\u001b\[[0-9;]*m/g, "");
}

let changed = 0;
let failed = 0;

for (const name of names) {
  let result;
  try {
    result = await diffComponent(name);
  } catch (error) {
    failed += 1;
    console.log(`${name}: failed (${error.message})`);
    continue;
  }

  const text = plain(result.output);
  if (result.code !== 0) {
    failed += 1;
    console.log(`${name}: failed`);
    console.log(text.trimEnd());
    console.log();
    continue;
  }

  if (/\((create|update|overwrite)\)/.test(text)) {
    changed += 1;
    console.log(`${name}: changes`);
    console.log(text.trimEnd());
    console.log();
  } else {
    console.log(`${name}: no changes`);
  }
}

console.log();
if (failed > 0) {
  console.log(`${failed} failed, ${changed} with changes.`);
  process.exit(1);
}
if (changed > 0) {
  console.log(`${changed} of ${names.length} with changes.`);
  process.exit(1);
}
console.log(`No changes in ${names.length} components.`);
