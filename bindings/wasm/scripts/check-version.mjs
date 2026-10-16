// Fails when the npm package version differs from the Cargo workspace version, or, given a tag
// (`v0.1.0`, `v0.2.0-beta.1`), when the tag is not `v` + that version.
// Usage: node bindings/wasm/scripts/check-version.mjs [tag]
import { readFileSync } from 'node:fs';

const read = (path) => readFileSync(new URL(path, import.meta.url), 'utf8');

const section = read('../../../Cargo.toml').split(/^\[workspace\.package\]\s*$/m)[1]?.split(/^\[/m)[0] ?? '';
const cargo = section.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
const npm = JSON.parse(read('../npm/package.json')).version;
const tag = process.argv[2];

const errors = [];
if (!cargo) errors.push('Cargo.toml has no [workspace.package] version');
if (cargo !== npm) errors.push(`bindings/wasm/npm/package.json is ${npm}, the Cargo workspace is ${cargo}`);
if (tag !== undefined && tag !== `v${npm}`) errors.push(`tag ${tag} does not match the package version ${npm}`);

if (errors.length) {
  for (const error of errors) console.error(`version: ${error}`);
  process.exit(1);
}
console.log(`version ${npm}${tag ? ` matches ${tag}` : ''}`);
