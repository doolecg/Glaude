// Writes THIRD_PARTY_NOTICES.txt: the licence of every Rust crate compiled into Glaude for Windows and every npm
// package bundled into its pages, copied from each package's own licence files. Run before a release build.
import { execFileSync } from 'node:child_process';
import { existsSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const LICENCE_FILE = /^(licen[cs]e|copying|notice)/i;

function licenceTexts(dir) {
  if (!existsSync(dir)) return [];
  return readdirSync(dir)
    .filter((f) => LICENCE_FILE.test(f))
    .sort()
    .map((f) => readFileSync(join(dir, f), 'utf8').trim());
}

// Only crates that end up in the Windows build, not build tools for other systems.
const metadata = JSON.parse(
  execFileSync('cargo', ['metadata', '--format-version', '1', '--filter-platform', 'x86_64-pc-windows-msvc', '--locked'], {
    cwd: join(root, 'src-tauri'),
    maxBuffer: 1 << 28,
  }),
);
const used = new Set(metadata.resolve.nodes.map((n) => n.id));
const crates = metadata.packages
  .filter((p) => p.source && used.has(p.id))
  .sort((a, b) => a.name.localeCompare(b.name) || a.version.localeCompare(b.version));

const npm = ['@tauri-apps/api', 'svelte'].map((name) => {
  const dir = join(root, 'node_modules', name);
  const pkg = JSON.parse(readFileSync(join(dir, 'package.json'), 'utf8'));
  return { name, version: pkg.version, license: pkg.license, dir };
});

const missing = [];
const sections = [...crates.map((p) => ({ name: p.name, version: p.version, license: p.license, dir: dirname(p.manifest_path) })), ...npm].map((p) => {
  const texts = licenceTexts(p.dir);
  if (!texts.length) missing.push(`${p.name} ${p.version}`);
  return `${p.name} ${p.version}\nLicence: ${p.license ?? 'see below'}\n\n${texts.length ? texts.join('\n\n') : `Licensed under ${p.license}; the package ships no licence file.`}`;
});

const head =
  'Glaude includes the open-source software listed below. Each part is the work of its own authors and is used under ' +
  'the licence shown with it, unchanged. The source code of every package is available from crates.io (Rust) and ' +
  'npmjs.com (JavaScript) under the name and version given. Glaude itself is MIT licensed; see LICENSE.\n';
writeFileSync(join(root, 'THIRD_PARTY_NOTICES.txt'), `${head}\n${'='.repeat(78)}\n\n${sections.join(`\n\n${'='.repeat(78)}\n\n`)}\n`);
console.log(`THIRD_PARTY_NOTICES.txt: ${sections.length} packages${missing.length ? `; no licence file in: ${missing.join(', ')}` : ''}`);
