#!/usr/bin/env node
import {
  chmodSync,
  copyFileSync,
  mkdirSync,
  readFileSync,
  writeFileSync,
} from "node:fs";
import { join } from "node:path";
import { parseArgs } from "node:util";

const ROOT = join(import.meta.dirname, "..");
const SCOPE = "@michalczmiel";
const MAIN = "recu";

const TARGETS = {
  "darwin-arm64": {
    rustTarget: "aarch64-apple-darwin",
    os: "darwin",
    cpu: "arm64",
  },
  "darwin-x64": {
    rustTarget: "x86_64-apple-darwin",
    os: "darwin",
    cpu: "x64",
  },
  "linux-x64": {
    rustTarget: "x86_64-unknown-linux-gnu",
    os: "linux",
    cpu: "x64",
  },
  "linux-arm64": {
    rustTarget: "aarch64-unknown-linux-gnu",
    os: "linux",
    cpu: "arm64",
  },
};

const VERSION = cargoVersion();

function cargoVersion() {
  const toml = readFileSync(join(ROOT, "Cargo.toml"), "utf8");
  const m = toml.match(/^version\s*=\s*"([^"]+)"/m);
  if (!m) throw new Error("version not found in Cargo.toml");
  return m[1];
}

function platformPkgJson(target, version) {
  const { os, cpu } = TARGETS[target];
  return {
    name: `${SCOPE}/${MAIN}-${target}`,
    version,
    description: `Prebuilt ${target} binary for recu`,
    license: "MIT",
    homepage: "https://github.com/michalczmiel/recu",
    repository: {
      type: "git",
      url: "git+https://github.com/michalczmiel/recu.git",
    },
    author: "Michał Czmiel",
    os: [os],
    cpu: [cpu],
    files: ["bin"],
  };
}

function platformReadme(target) {
  return `# ${SCOPE}/${MAIN}-${target}

Prebuilt ${target} binary for [${SCOPE}/${MAIN}](https://www.npmjs.com/package/${SCOPE}/${MAIN}). Don't depend on this directly — install \`${SCOPE}/${MAIN}\` instead.
`;
}

function buildPlatform(target) {
  const cfg = TARGETS[target];
  if (!cfg) {
    throw new Error(
      `unknown target: ${target}. known: ${Object.keys(TARGETS).join(", ")}`,
    );
  }
  const dir = join(ROOT, "npm", `${MAIN}-${target}`);
  mkdirSync(join(dir, "bin"), { recursive: true });
  writeFileSync(
    join(dir, "package.json"),
    `${JSON.stringify(platformPkgJson(target, VERSION), null, 2)}\n`,
  );
  writeFileSync(join(dir, "README.md"), platformReadme(target));
  const src = join(ROOT, "target", cfg.rustTarget, "release", MAIN);
  const dst = join(dir, "bin", MAIN);
  copyFileSync(src, dst);
  chmodSync(dst, 0o755);
  console.log(`built npm/${MAIN}-${target} v${VERSION}`);
}

function syncMain() {
  const path = join(ROOT, "npm", MAIN, "package.json");
  const pkg = JSON.parse(readFileSync(path, "utf8"));
  pkg.version = VERSION;
  pkg.optionalDependencies = Object.fromEntries(
    Object.keys(TARGETS).map((t) => [`${SCOPE}/${MAIN}-${t}`, VERSION]),
  );
  writeFileSync(path, `${JSON.stringify(pkg, null, 2)}\n`);
  console.log(`synced npm/${MAIN}/package.json to v${VERSION}`);
}

const { values } = parseArgs({
  options: {
    target: { type: "string", short: "t" },
  },
  strict: true,
});

if (values.target) {
  buildPlatform(values.target);
} else {
  syncMain();
}
