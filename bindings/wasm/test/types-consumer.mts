// Type-checked (never run) from the clean consumer: the declarations resolve through `exports`
// and describe the API that node-consumer.mjs and browser-page.mjs exercise.
import { load, type BuildOutput, type Cdr, type Sdk, type SignOutput, type SoapRequest, type TicketOutcome, type ZipOutput, type RestRequest, type TokenOutcome, type ValidityOutcome, type GreRequest, type GreSendRequest, type GreTokenOutcome, type GreSendOutcome, type GreTicketOutcome, type SendOutcome, type Diagnosis, type DiagnosisField } from '@kembec/facturacion-electronica-sunat-sdk';
import wasm from '@kembec/facturacion-electronica-sunat-sdk/wasm';
import suffixed from '@kembec/facturacion-electronica-sunat-sdk/sdk.wasm';

const sdk: Sdk = await load();
const same: Sdk = await load(wasm);
const bundled: Sdk = await load(suffixed);
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
const greTokenCall: GreRequest = JSON.parse(sdk.greTokenRequest('id', 'secret', '20000000001USER', 'pass', 'https://auth.test'));
const greToken: GreTokenOutcome = JSON.parse(sdk.greTokenResponse(200, new Uint8Array(), 1760620000));
const reusable: boolean = sdk.greTokenUsable(JSON.stringify(greToken), 1760620000);
const greSendCall: GreSendRequest = JSON.parse(sdk.greSendRequest('{}', 'token', 'https://api.test'));
const greTicket: GreSendOutcome = JSON.parse(sdk.greSendResponse(200, new Uint8Array()));
const greStatus: GreTicketOutcome = JSON.parse(sdk.greTicketResponse(200, new Uint8Array(), 'a.b.c'));
const greLate: GreTicketOutcome = JSON.parse(sdk.greTimeout('ticket', 30, 'a.b.c'));
const askAgain: boolean = greStatus.outcome === 'ticket_status' && greStatus.state === 'in_process' && greTicket.outcome === 'ticket' && greLate.outcome === 'no_verdict';
export const greUsed = [greTokenCall.url, greSendCall.hash_zip, reusable, askAgain, sdk.greTicketRequest('a.b.c', 'token', 'https://api.test')];
const sent: SendOutcome = JSON.parse(sdk.soapResponse('send', '{}', 200, new Uint8Array()));
const diagnosed: Diagnosis = JSON.parse(sdk.diagnose('03', '2800', 'x'));
const field: DiagnosisField | undefined = sent.diagnosis[0]?.fields[0];
const pointed: (string | null)[] = [diagnosed.code, diagnosed.detail?.node ?? null, field?.input_path ?? null, field?.xpath ?? null];
const sure: boolean = field?.certainty === 'fuente';
export const diagnosisUsed = [pointed, sure];
export const used = [bundled, tokenCall.url, queryCall.headers, decided, fromUrl, signed.digest_value, signed.signature_value, built.file_name, named, zipped, read, post.body, again];

// @ts-expect-error certainty is fuente or inferido
const unsure: DiagnosisField['certainty'] = 'maybe';
// @ts-expect-error signatureId is a string
sdk.sign(built.xml, 'cert', 'key', 1);
// @ts-expect-error the package does no network I/O: there is no send
sdk.send();
// @ts-expect-error zip is not an exchange
sdk.soapRequest('zip', '{}', 'user', 'password');
// @ts-expect-error a validity timeout belongs to the token or the query call
sdk.validityTimeout('send', 30);
// @ts-expect-error a GRE timeout belongs to the token, send or ticket call
sdk.greTimeout('query', 30);
// @ts-expect-error the token base has no default: GRE has no test environment
sdk.greTokenRequest('id', 'secret', '20000000001USER', 'pass');
