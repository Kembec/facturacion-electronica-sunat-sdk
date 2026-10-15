// Builds and signs every fixture through the wasm binding and compares with the vectors.
// Run after: wasm-bindgen --target nodejs --out-dir bindings/wasm/pkg <the .wasm>
// Usage: node bindings/wasm/test/sign-fixture.mjs
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, '..', '..', '..');
const { build, sign } = createRequire(import.meta.url)(join(here, '..', 'pkg', 'sunat_sdk_wasm.js'));

const read = (...parts) => readFileSync(join(root, ...parts), 'utf8');
const elementText = (xml, element) => {
  const match = xml.match(new RegExp(`<${element}>([^<]*)<`));
  assert.ok(match, `no ${element}`);
  return match[1].replace(/\s+/g, '');
};

const cert = read('tests', 'vectors', 'test-cert', 'test-cert.pem');
const key = read('tests', 'vectors', 'test-cert', 'test-key.pem');
const cases = readdirSync(join(root, 'fixtures'))
  .filter((name) => name.endsWith('.json'))
  .map((name) => name.slice(0, -'.json'.length))
  .sort();
assert.equal(cases.length, 12, 'fixtures');

for (const name of cases) {
  const built = JSON.parse(build(read('fixtures', `${name}.json`)));
  const signed = JSON.parse(sign(built.xml, cert, key));
  const expected = read('tests', 'vectors', `${name}.signed.xml`);
  assert.equal(signed.digest_value, elementText(expected, 'ds:DigestValue'), `${name}: digest_value`);
  assert.equal(signed.signature_value, elementText(expected, 'ds:SignatureValue'), `${name}: signature_value`);
  console.log(`ok ${name}`);
}

const custom = JSON.parse(sign(read('tests', 'vectors', `${cases[0]}.unsigned.xml`), cert, key, 'customSig'));
assert.ok(custom.xml.includes('Id="customSig"'), 'custom signature_id');
assert.throws(() => build('{not json'), Error, 'invalid JSON throws');
assert.throws(() => sign('<a/>', cert, 'not a key'), Error, 'invalid key throws');
console.log(`ok ${cases.length} fixtures signed through wasm`);
