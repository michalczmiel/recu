#!/usr/bin/env node
import { parseArgs } from "node:util";
import { readFile, writeFile } from "node:fs/promises";
import { promisify } from "node:util";
import { exec as syncExec } from "node:child_process";
import { join } from "node:path";

const exec = promisify(syncExec);

async function updateCargoVersion(newVersion) {
  const content = await readFile("Cargo.toml", "utf8");

  // Targets the first 'version =' in the file (the package version)
  const updated = content.replace(
    /^version\s*=\s*".*"/m,
    `version = "${newVersion}"`,
  );

  await writeFile("Cargo.toml", updated);

  await exec("cargo update --workspace");
}

async function updateNpmVersion(newVersion) {
  const path = join("npm", "recu", "package.json");

  const mainFile = await readFile(path, "utf8");
  const mainPackageData = JSON.parse(mainFile);

  const currentVersion = mainPackageData.version;

  const updatedPackage = mainFile.replaceAll(currentVersion, newVersion);

  await writeFile(path, updatedPackage);
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

await Promise.all([
  updateCargoVersion(values.version),
  updateNpmVersion(values.version),
]);
