#!/usr/bin/env node
const { spawnSync } = require("node:child_process");

const SUPPORTED = new Set([
  "darwin-arm64",
  "darwin-x64",
  "linux-x64",
  "linux-arm64",
]);
const target = `${process.platform}-${process.arch}`;
const platformPkg = `@michalczmiel/recu-${target}`;

let binary;
try {
  binary = require.resolve(`${platformPkg}/bin/recu`);
} catch {
  if (!SUPPORTED.has(target)) {
    console.error(
      `[recu] No prebuilt binary available for ${target}.\n` +
        `Supported targets: ${[...SUPPORTED].join(", ")}.\n` +
        `Build from source instead:\n` +
        `  cargo install recu`,
    );
  } else {
    console.error(
      `[recu] Prebuilt binary for ${target} was not installed.\n` +
        `This usually means optional dependencies were skipped (e.g. \`npm install --no-optional\`,\n` +
        `\`yarn --ignore-optional\`, or a node_modules moved between machines/containers).\n` +
        `Fix by reinstalling with optional deps enabled:\n` +
        `  npm install -g @michalczmiel/recu --include=optional\n` +
        `Or build from source:\n` +
        `  cargo install recu`,
    );
  }
  process.exit(1);
}

const result = spawnSync(binary, process.argv.slice(2), { stdio: "inherit" });

if (result.error) {
  console.error(result.error);
  process.exit(1);
}

if (result.signal) {
  process.kill(process.pid, result.signal);
}

process.exit(result.status ?? 1);
