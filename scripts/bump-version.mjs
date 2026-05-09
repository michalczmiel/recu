#!/usr/bin/env node
import { parseArgs } from "node:util";
import { readFile, writeFile } from "node:fs/promises";
import { promisify } from "node:util";
import { exec as syncExec } from "node:child_process";

const exec = promisify(syncExec);

async function updateCargoVersion(newVersion) {
  let content = await readFile("Cargo.toml", "utf8");
  // Targets the first 'version =' in the file (the package version)
  content = content.replace(
    /^version\s*=\s*".*"/m,
    `version = "${newVersion}"`,
  );

  await writeFile("Cargo.toml", content);

  await exec("cargo update --workspace");
}

const { values } = parseArgs({
  options: {
    version: {
      type: "string",
    },
  },
});

if (!values.version) {
  console.error("Please provide a version");
  process.exit(1);
}

await updateCargoVersion(values.version);
