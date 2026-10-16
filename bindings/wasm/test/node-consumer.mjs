// Runs from a clean project that installed the packed tarball, so the package is imported by its
// name and resolved through its `exports` (`node` condition), as a user would get it.
// Usage: node <consumer>/node-consumer.mjs <repository root> [--source]
import { spawnSync } from 'node:child_process';
import { readFileSync, readdirSync } from 'node:fs';
import { createRequire } from 'node:module';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { load } from '@kembec/facturacion-electronica-sunat-sdk';
import { runVectors, selectCases } from './vectors.mjs';

const PACKAGE = '@kembec/facturacion-electronica-sunat-sdk';
const [root, mode] = process.argv.slice(2);
if (!root) throw new Error('usage: node node-consumer.mjs <repository root> [--source]');

const resolve = createRequire(import.meta.url).resolve;
const fromSource = mode === '--source';
// --source: the caller hands load() a compiled module, read through the `./wasm` export.
const source = fromSource ? new WebAssembly.Module(readFileSync(resolve(`${PACKAGE}/wasm`))) : undefined;
const sdk = await load(source);
if ((await load()) !== sdk) throw new Error('load() returns the same SDK on every call');

await runVectors({
  sdk,
  cases: selectCases(readdirSync(join(root, 'fixtures'))),
  read: async (path) => readFileSync(join(root, path), 'utf8'),
  log: (line) => console.log(line),
});
// The exchange with SUNAT goes through the caller's fetch: the package gives the POST and reads the
// answer as the CLI would. A 401 to getStatus is no verdict, with the same ticket to ask again.
const ticket = JSON.stringify({ ticket: '1760620000123' });
const post = JSON.parse(sdk.soapRequest('ticket', ticket, '20100066603MODDATOS', 'moddatos'));
if (post.method !== 'POST' || post.headers.SOAPAction !== '"urn:getStatus"' || !post.body.includes('<ticket>1760620000123</ticket>')) {
  throw new Error(`soapRequest: ${JSON.stringify(post.headers)}`);
}
const unauthorized = JSON.parse(sdk.soapResponse('ticket', ticket, 401, new Uint8Array()));
if (unauthorized.outcome !== 'no_verdict' || unauthorized.http_status !== 401 || unauthorized.ticket !== '1760620000123') {
  throw new Error(`soapResponse 401: ${JSON.stringify(unauthorized)}`);
}
let sendTimeout;
try {
  sdk.soapTimeout('send', JSON.stringify({ name: '20100066603-01-F001-1', xml: '<Invoice/>' }), 45);
} catch (error) {
  sendTimeout = error.message;
}
if (!sendTimeout?.includes('the outcome is unknown')) throw new Error(`soapTimeout send: ${sendTimeout}`);
console.log(`ok node exchange: soapRequest, soapResponse 401 -> no_verdict, soapTimeout send throws`);
console.log(`ok node ${process.version} ${fromSource ? 'load(WebAssembly.Module)' : 'load()'} via ${resolve(PACKAGE)}`);

if (!fromSource) {
  const child = spawnSync(process.execPath, [fileURLToPath(import.meta.url), root, '--source'], { stdio: 'inherit' });
  if (child.status !== 0) process.exit(child.status ?? 1);
}
