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
// The 2800 the beta answered to a boleta: the outcome names the input field, and diagnose() alone agrees.
const boleta = JSON.stringify({ name: '20100066603-03-B001-1', xml: '<Invoice/>' });
const rejected = JSON.parse(sdk.soapResponse('send', boleta, 200, new Uint8Array(readFileSync(join(root, 'bindings/wasm/test/data/beta-2800-fault.xml')))));
const [found] = rejected.diagnosis ?? [];
if (rejected.code !== '2800' || rejected.diagnosis.length !== 1 || found.detail?.node !== 'cbc:ID/schemeID' || found.detail.value !== '1' || found.fields[0]?.input_path !== 'customer.document_type') {
  throw new Error(`soapResponse 2800: ${JSON.stringify(rejected.diagnosis)}`);
}
if (JSON.stringify(JSON.parse(sdk.diagnose('03', '2800', rejected.message))) !== JSON.stringify(found)) {
  throw new Error(`diagnose 2800: ${sdk.diagnose('03', '2800', rejected.message)}`);
}
if ('diagnosis' in unauthorized) throw new Error('no verdict has no diagnosis');
console.log('ok node diagnosis: beta 2800 -> customer.document_type (cbc:ID/schemeID = 1)');
// Consulta Integrada: token, then query, both through the caller's fetch; a 401 is no verdict.
const tokenCall = JSON.parse(sdk.validityTokenRequest('c04864ce-f454-48e1-b41d-8730e575b905', 'OpzHz4pn8D1frRrv5iDklA=='));
if (!tokenCall.url.endsWith('/c04864ce-f454-48e1-b41d-8730e575b905/oauth2/token/') || !tokenCall.body.includes('client_secret=OpzHz4pn8D1frRrv5iDklA%3D%3D')) {
  throw new Error(`validityTokenRequest: ${tokenCall.url}`);
}
const encoder = new TextEncoder();
const token = JSON.parse(sdk.validityTokenResponse(200, encoder.encode('{"access_token":"t0k","token_type":"JWT","expires_in":3600}')));
const queryInput = JSON.stringify({ ruc: '20100066603', document_type: '03', series: 'B001', number: 7, issue_date: '2026-10-15', total: '118' });
const queryCall = JSON.parse(sdk.validityRequest(queryInput, token.access_token));
if (queryCall.headers.Authorization !== 'Bearer t0k' || JSON.parse(queryCall.body).monto !== '118.00') throw new Error(`validityRequest: ${queryCall.body}`);
const reported = JSON.parse(sdk.validityResponse(200, encoder.encode('{"success":true,"message":"ok","data":{"estadoCp":"1","observaciones":[]}}')));
const refused = JSON.parse(sdk.validityResponse(401, new Uint8Array()));
if (reported.state !== 'accepted' || refused.outcome !== 'no_verdict' || refused.stage !== 'query') {
  throw new Error(`validityResponse: ${JSON.stringify([reported, refused])}`);
}
console.log('ok node validity: token, query, 401 -> no_verdict');
// GRE: token, send and ticket through the caller's fetch, replaying the core's recorded exchanges
// (crates/sunat-sdk/tests/data/gre, none accepted by SUNAT: there is no test environment).
const gre = (name) => new Uint8Array(readFileSync(join(root, `crates/sunat-sdk/tests/data/gre/${name}.json`)));
const greTokenCall = JSON.parse(sdk.greTokenRequest('abc-123', 's/e=c', '20000000001MODUSER', 'p&ss', 'https://auth.test/v1/clientessol'));
if (greTokenCall.url !== 'https://auth.test/v1/clientessol/abc-123/oauth2/token/' || !greTokenCall.body.startsWith('grant_type=password&')) {
  throw new Error(`greTokenRequest: ${greTokenCall.url}`);
}
const greToken = JSON.parse(sdk.greTokenResponse(200, gre('derived-token-200'), 1000));
if (greToken.expires_at !== 4600 || !sdk.greTokenUsable(JSON.stringify(greToken), 3999) || sdk.greTokenUsable(JSON.stringify(greToken), 4000)) {
  throw new Error(`greTokenResponse: ${JSON.stringify(greToken.expires_at)}`);
}
let millis;
try {
  sdk.greTokenResponse(200, gre('derived-token-200'), Date.now());
} catch (error) {
  millis = error.message;
}
if (!millis?.includes('Unix seconds')) throw new Error(`greTokenResponse with milliseconds: ${millis}`);
const guide = JSON.stringify({ name: '20000000001-09-T001-1', xml: '<DespatchAdvice/>' });
const greSendCall = JSON.parse(sdk.greSendRequest(guide, greToken.access_token, 'https://api.test/v1'));
const archivo = JSON.parse(greSendCall.body).archivo;
if (greSendCall.headers.Authorization !== 'Bearer fake.token.for-tests' || archivo.hashZip !== greSendCall.hash_zip || archivo.nomArchivo !== '20000000001-09-T001-1.zip') {
  throw new Error(`greSendRequest: ${greSendCall.url}`);
}
const greTicket = JSON.parse(sdk.greSendResponse(200, gre('fork-send-200')));
const greRejected = JSON.parse(sdk.greSendResponse(422, gre('fork-send-422')));
if (greTicket.ticket !== 'a.b.c' || greRejected.outcome !== 'api_error' || greRejected.errors[0].code !== '501') {
  throw new Error(`greSendResponse: ${JSON.stringify([greTicket, greRejected])}`);
}
const greQuery = JSON.parse(sdk.greTicketRequest(greTicket.ticket, greToken.access_token, 'https://api.test/v1'));
if (greQuery.method !== 'GET' || greQuery.url !== 'https://api.test/v1/contribuyente/gem/comprobantes/envios/a.b.c') throw new Error(`greTicketRequest: ${greQuery.url}`);
const states = ['0', '98', '99'].map((code) => JSON.parse(sdk.greTicketResponse(200, gre(`fork-ticket-${code}`), 'a.b.c')));
const [processed, inProcess, withErrors] = states;
if (processed.state !== 'processed' || processed.cdr?.reference_id !== 'T001-1' || processed.cdr.class !== 'observed') throw new Error(`ticket 0: ${processed.state}`);
if (inProcess.state !== 'in_process' || inProcess.cdr_generated !== false || 'cdr' in inProcess) throw new Error(`ticket 98: ${JSON.stringify(inProcess)}`);
if (withErrors.state !== 'processed_with_errors' || withErrors.error?.code !== '1345' || withErrors.error.class !== 'exception') throw new Error(`ticket 99: ${JSON.stringify(withErrors)}`);
const greLate = JSON.parse(sdk.greTimeout('ticket', 30, 'a.b.c'));
let sendLost;
try {
  sdk.greSendResponse(502, encoder.encode('<html/>'));
} catch (error) {
  sendLost = error.message;
}
if (greLate.outcome !== 'no_verdict' || greLate.ticket !== 'a.b.c' || !sendLost?.includes('may have arrived')) throw new Error(`gre silence: ${sendLost}`);
console.log('ok node gre: token, send, ticket 0/98/99, 422 -> api_error, silence');
console.log(`ok node ${process.version} ${fromSource ? 'load(WebAssembly.Module)' : 'load()'} via ${resolve(PACKAGE)}`);

if (!fromSource) {
  const child = spawnSync(process.execPath, [fileURLToPath(import.meta.url), root, '--source'], { stdio: 'inherit' });
  if (child.status !== 0) process.exit(child.status ?? 1);
}
