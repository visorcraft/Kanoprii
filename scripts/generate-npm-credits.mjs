#!/usr/bin/env node
/**
 * Emit npm package credit rows and license-text sections for Kanoprii's
 * shipped frontend dependencies (root package.json "dependencies" only).
 */
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const pkg = JSON.parse(fs.readFileSync(path.join(root, 'package.json'), 'utf8'));
const lock = JSON.parse(fs.readFileSync(path.join(root, 'package-lock.json'), 'utf8'));

const PROJECT_URLS = {
  react: 'https://react.dev/',
  'react-dom': 'https://react.dev/',
  '@tauri-apps/api': 'https://github.com/tauri-apps/tauri',
  '@tauri-apps/plugin-dialog': 'https://github.com/tauri-apps/plugins-workspace',
  '@tauri-apps/plugin-process': 'https://github.com/tauri-apps/plugins-workspace',
  '@tauri-apps/plugin-updater': 'https://github.com/tauri-apps/plugins-workspace',
  marked: 'https://marked.js.org/',
  html2canvas: 'https://html2canvas.hertzen.com/',
};

const MIT_TEXT = fs.readFileSync(path.join(root, 'LICENSES/MIT.txt'), 'utf8').trim();
const APACHE_TEXT = fs.readFileSync(path.join(root, 'LICENSES/Apache-2.0.txt'), 'utf8').trim();

const licenseBodies = {
  'MIT License': MIT_TEXT,
  'Apache License 2.0': APACHE_TEXT,
  'MIT OR Apache-2.0': `${MIT_TEXT}\n\n${'='.repeat(64)}\n\n${APACHE_TEXT}`,
};

function sectionTitle(license) {
  if (license === 'MIT') return 'MIT License';
  if (license === 'Apache-2.0') return 'Apache License 2.0';
  if (license.includes('MIT') && license.includes('Apache')) return 'MIT OR Apache-2.0';
  return license;
}

function packageEntry(name) {
  return lock.packages[`node_modules/${name}`];
}

/** @type {{ name: string, version: string, license: string, url: string }[]} */
const rows = [];
for (const name of Object.keys(pkg.dependencies).sort()) {
  const entry = packageEntry(name);
  if (!entry) {
    throw new Error(`Missing lockfile entry for dependency ${name}`);
  }
  const license = String(entry.license ?? 'UNKNOWN');
  rows.push({
    name,
    version: String(entry.version),
    license: license.includes('MIT') && license.includes('Apache') ? 'MIT OR Apache-2.0' : license,
    url: PROJECT_URLS[name] ?? `https://www.npmjs.com/package/${name}`,
  });
}

const grouped = new Map();
for (const row of rows) {
  const title = sectionTitle(row.license);
  if (!grouped.has(title)) grouped.set(title, []);
  grouped.get(title).push(row);
}

let md = `<!-- SPDX-FileCopyrightText: 2026 VisorCraft LLC -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->
# Shipped npm packages

These are the direct npm packages bundled into Kanoprii's React frontend.
Dev-only tooling under \`e2e/\` is intentionally omitted because it is not
shipped in release builds.

## Packages in use

`;
for (const row of rows) {
  md += `- **${row.name}** ${row.version} - ${row.license}\n`;
}

md += '\n---\n\n## npm License Texts\n\n';

for (const [title, packages] of [...grouped.entries()].sort(([a], [b]) => a.localeCompare(b))) {
  md += `### ${title}\n\nUsed by:\n`;
  for (const row of packages.sort((a, b) => a.name.localeCompare(b.name))) {
    md += `- [\`${row.name} ${row.version}\`](${row.url})\n`;
  }
  md += '\n```\n';
  md += (licenseBodies[title] ?? `License expression: ${title}\n\nFull license text is not bundled for this expression.`);
  md += '\n```\n\n---\n\n';
}

const jsonPath = path.join(root, 'docs/credits-npm.json');
const mdPath = path.join(root, 'docs/credits-npm.md');
fs.writeFileSync(jsonPath, `${JSON.stringify(rows, null, 2)}\n`);
fs.writeFileSync(mdPath, md);
console.log(`Wrote ${mdPath} (${rows.length} packages)`);
console.log(`Wrote ${jsonPath}`);
