import { readFile, readdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const lockPath = path.join(repositoryRoot, "package-lock.json");
const outputPath = path.join(repositoryRoot, "THIRD_PARTY_NPM_LICENSES.html");

const escapeHtml = (value) =>
  value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");

const lock = JSON.parse(await readFile(lockPath, "utf8"));
const productionPackages = Object.entries(lock.packages)
  .filter(([packagePath, metadata]) =>
    packagePath.startsWith("node_modules/") && metadata.dev !== true,
  )
  .map(([packagePath, metadata]) => ({ packagePath, metadata }))
  .sort((left, right) => left.packagePath.localeCompare(right.packagePath));

const sections = [];

for (const { packagePath, metadata } of productionPackages) {
  const directory = path.join(repositoryRoot, packagePath);
  const packageJson = JSON.parse(
    await readFile(path.join(directory, "package.json"), "utf8"),
  );
  const filenames = (await readdir(directory))
    .filter((filename) => /^(licen[cs]e|copying|notice)/i.test(filename))
    .sort();

  if (filenames.length === 0) {
    throw new Error(`No license or notice file found for ${packageJson.name}`);
  }

  const files = [];
  for (const filename of filenames) {
    files.push({
      filename,
      text: await readFile(path.join(directory, filename), "utf8"),
    });
  }

  const repository =
    typeof packageJson.repository === "string"
      ? packageJson.repository
      : packageJson.repository?.url;
  const homepage = packageJson.homepage ?? repository ?? "";
  const license = packageJson.license ?? metadata.license ?? "See included files";

  sections.push(`
      <section>
        <h2>${escapeHtml(packageJson.name)} ${escapeHtml(packageJson.version)}</h2>
        <p>Declared license: ${escapeHtml(String(license))}</p>
        ${
          homepage
            ? `<p><a href="${escapeHtml(String(homepage).replace(/^git\+/, ""))}">Project website</a></p>`
            : ""
        }
        ${files
          .map(
            ({ filename, text }) => `
        <h3>${escapeHtml(filename)}</h3>
        <pre>${escapeHtml(text)}</pre>`,
          )
          .join("")}
      </section>`);
}

const html = `<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>PixChisel npm third-party licenses</title>
    <style>
      :root { color-scheme: light dark; }
      body { font: 15px/1.5 system-ui, sans-serif; margin: 0; }
      main { margin: 0 auto; max-width: 960px; padding: 32px; }
      pre { border: 1px solid #8886; border-radius: 8px; max-height: 320px;
        overflow: auto; padding: 16px; white-space: pre-wrap; }
      a { color: inherit; }
    </style>
  </head>
  <body>
    <main>
      <h1>PixChisel npm third-party licenses</h1>
      <p>
        Generated from the production dependency graph locked by package-lock.json.
        Development-only packages are excluded.
      </p>
      ${sections.join("\n")}
    </main>
  </body>
</html>
`;

await writeFile(outputPath, html, "utf8");
console.log(
  `Generated ${path.basename(outputPath)} for ${productionPackages.length} packages.`,
);
