import { readFile, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";

const command = process.argv[2];
if (command !== "sync" && command !== "check") {
  console.error("Usage: node scripts/version.mjs <sync|check>");
  process.exit(1);
}

const projectRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const packagePath = path.join(projectRoot, "package.json");
const tauriConfigPath = path.join(projectRoot, "src-tauri", "tauri.conf.json");
const cargoManifestPath = path.join(projectRoot, "src-tauri", "Cargo.toml");

const packageJson = JSON.parse(await readFile(packagePath, "utf8"));
const canonicalVersion = packageJson.version;

if (!isSemanticVersion(canonicalVersion)) {
  console.error(`package.json contains an invalid semantic version: ${canonicalVersion}`);
  process.exit(1);
}

const tauriConfig = JSON.parse(await readFile(tauriConfigPath, "utf8"));
const cargoManifest = await readFile(cargoManifestPath, "utf8");
const cargoVersion = readCargoPackageVersion(cargoManifest);

if (command === "check") {
  const mismatches = [];
  if (tauriConfig.version !== canonicalVersion) {
    mismatches.push(`src-tauri/tauri.conf.json: ${tauriConfig.version}`);
  }
  if (cargoVersion !== canonicalVersion) {
    mismatches.push(`src-tauri/Cargo.toml: ${cargoVersion}`);
  }

  if (mismatches.length > 0) {
    console.error(`Version metadata does not match package.json (${canonicalVersion}):`);
    mismatches.forEach((mismatch) => console.error(`- ${mismatch}`));
    console.error("Run npm run version:sync to align generated version metadata.");
    process.exit(1);
  }

  console.log(`Version metadata is aligned at ${canonicalVersion}.`);
  process.exit(0);
}

tauriConfig.version = canonicalVersion;
await writeFile(tauriConfigPath, `${JSON.stringify(tauriConfig, null, 2)}\n`);
await writeFile(
  cargoManifestPath,
  replaceCargoPackageVersion(cargoManifest, canonicalVersion),
);
console.log(`Synchronized PixChisel version metadata to ${canonicalVersion}.`);

function isSemanticVersion(version) {
  return typeof version === "string" && /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(version);
}

function readCargoPackageVersion(manifest) {
  const packageSection = manifest.match(/^\[package\]\s*$([\s\S]*?)(?=^\[|(?![\s\S]))/m)?.[1];
  const version = packageSection?.match(/^version\s*=\s*"([^"]+)"\s*$/m)?.[1];
  if (!version) {
    throw new Error("Could not find [package].version in src-tauri/Cargo.toml.");
  }
  return version;
}

function replaceCargoPackageVersion(manifest, version) {
  let replaced = false;
  const updated = manifest.replace(
    /(^\[package\]\s*$[\s\S]*?^version\s*=\s*")[^"]+("\s*$)/m,
    (_match, prefix, suffix) => {
      replaced = true;
      return `${prefix}${version}${suffix}`;
    },
  );
  if (!replaced) {
    throw new Error("Could not update [package].version in src-tauri/Cargo.toml.");
  }
  return updated;
}
