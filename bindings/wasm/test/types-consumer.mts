// Type-checked (never run) from the clean consumer: the declarations resolve through `exports`
// and describe the API that node-consumer.mjs and browser-page.mjs exercise.
import { load, type BuildOutput, type Cdr, type Sdk, type SignOutput, type SoapRequest, type TicketOutcome, type ZipOutput, type RestRequest, type TokenOutcome, type ValidityOutcome } from '@kembec/facturacion-electronica-sunat-sdk';
import wasm from '@kembec/facturacion-electronica-sunat-sdk/wasm';

const sdk: Sdk = await load();
const same: Sdk = await load(wasm);
const fromUrl: Promise<Sdk> = load(new URL('https://example.com/sunat_sdk_wasm_bg.wasm'));
const built: BuildOutput = JSON.parse(sdk.build('{}'));
const signed: SignOutput = JSON.parse(same.sign(built.xml, 'cert', 'key'));
const named: string = sdk.sign(built.xml, 'cert', 'key', 'customSig');
const zipped: ZipOutput = JSON.parse(sdk.zip('{}'));
const read: Cdr = JSON.parse(sdk.cdr('{}'));
const post: SoapRequest = JSON.parse(sdk.soapRequest('ticket', '{}', 'user', 'password'));
const answer: TicketOutcome = JSON.parse(sdk.soapResponse('ticket', '{}', 401, new Uint8Array()));
const late: TicketOutcome = JSON.parse(sdk.soapTimeout('ticket', '{}', 45));
const again: boolean = answer.outcome === 'no_verdict' && answer.http_status === 401 && late.outcome === 'no_verdict';
const tokenCall: RestRequest = JSON.parse(sdk.validityTokenRequest('id', 'secret'));
const token: TokenOutcome = JSON.parse(sdk.validityTokenResponse(200, new Uint8Array()));
const queryCall: RestRequest = JSON.parse(sdk.validityRequest('{}', 'token', 'https://api.test'));
const validity: ValidityOutcome = JSON.parse(sdk.validityResponse(200, new Uint8Array()));
const stateless: ValidityOutcome = JSON.parse(sdk.validityTimeout('query', 30));
const decided: boolean = validity.outcome === 'validity' && validity.state === 'not_reported' && token.outcome === 'token' && stateless.outcome === 'no_verdict';
export const used = [tokenCall.url, queryCall.headers, decided, fromUrl, signed.digest_value, signed.signature_value, built.file_name, named, zipped, read, post.body, again];

// @ts-expect-error signatureId is a string
sdk.sign(built.xml, 'cert', 'key', 1);
// @ts-expect-error the package does no network I/O: there is no send
sdk.send();
// @ts-expect-error zip is not an exchange
sdk.soapRequest('zip', '{}', 'user', 'password');
// @ts-expect-error a validity timeout belongs to the token or the query call
sdk.validityTimeout('send', 30);
