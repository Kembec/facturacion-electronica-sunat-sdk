/** What `build` returns, once parsed with `JSON.parse`. */
export interface BuildOutput {
  /** `<RUC>-<type>-<series>-<number>`, the name SUNAT expects without extension. */
  file_name: string;
  /** Unsigned UBL 2.1 XML. */
  xml: string;
}

/** What `sign` returns, once parsed with `JSON.parse`. */
export interface SignOutput {
  /** Signed XML, with the signature inside `ext:UBLExtensions`. */
  xml: string;
  digest_value: string;
  signature_value: string;
}

/** What `zip` returns, once parsed. */
export interface ZipOutput {
  /** `<name>.zip`. */
  file_name: string;
  zip_base64: string;
}

/** A CDR (constancia de recepción), as `cdr` returns it and as it comes inside an outcome. */
export interface Cdr {
  reference_id: string;
  response_code: string;
  description: string;
  notes: string[];
  /** `null` for a code outside the manual's ranges. */
  class: 'accepted' | 'observed' | 'exception' | 'already_registered' | 'number_voided_or_rejected' | 'rejected' | 'void_already_reported' | 'unknown' | null;
}

/** The four SUNAT web-service calls, named as the CLI commands. */
export type Exchange = 'send' | 'summary' | 'ticket' | 'status';

/** What `soapRequest` returns, once parsed: the options for `fetch(endpoint, options)`. */
export interface SoapRequest {
  method: 'POST';
  headers: { 'Content-Type': string; SOAPAction: string };
  /** The SOAP envelope, with the SOL credentials in its WS-Security header: never log it. */
  body: string;
}

/** A SOAP fault from SUNAT: a result, not an exception. */
export interface FaultOutcome {
  outcome: 'fault';
  code: string;
  message: string;
  class: Cdr['class'];
}

/**
 * `ticket` or `status` without an answer (an HTTP status other than 200/500, or the wait ran out).
 * It says nothing about the document: ask again with the same data, never send again because of it.
 */
export interface NoVerdictOutcome {
  outcome: 'no_verdict';
  reason: 'http_status' | 'timeout';
  http_status: number | null;
  message: string;
  /** The same ticket, for `ticket`. */
  ticket?: string;
}

/** `send`: the CDR or a fault. */
export type SendOutcome = { outcome: 'cdr'; cdr: Cdr; cdr_zip_base64: string } | FaultOutcome;
/** `summary`: the ticket, the only handle on the outcome (store it first), or a fault. */
export type SummaryOutcome = { outcome: 'ticket'; ticket: string } | FaultOutcome;
/** `ticket`: `state` is `processed` (0), `in_process` (98: ask again) or `processed_with_errors` (99). */
export type TicketOutcome =
  | { outcome: 'ticket_status'; status_code: string; state: 'processed' | 'in_process' | 'processed_with_errors' | null; cdr?: Cdr; cdr_zip_base64?: string }
  | FaultOutcome
  | NoVerdictOutcome;
/** `status` (getStatusCdr). */
export type StatusOutcome =
  | { outcome: 'status'; status_code: string; status_message: string; cdr?: Cdr; cdr_zip_base64?: string }
  | FaultOutcome
  | NoVerdictOutcome;

/** A Consulta Integrada call as `fetch(url, options)` takes it; `url` included. */
export interface RestRequest {
  method: 'POST';
  url: string;
  headers: Record<string, string>;
  /** Carries the client secret (token) or goes with the token in `Authorization` (query): never log it. */
  body: string;
}

/** Input of `validityRequest`, the CLI's `validity` stdin. Types 01, 03, 07 and 08 only. */
export interface ValidityInput {
  /** The issuer's RUC. */
  ruc: string;
  document_type: '01' | '03' | '07' | '08';
  series: string;
  number: number | string;
  /** `YYYY-MM-DD`, or the document's ISO 8601 `issue_date`. */
  issue_date: string;
  /** The voucher's total (PayableAmount), up to 2 decimals, as a string. */
  total: string;
  /** The RUC that asks, in the URL; defaults to `ruc`. */
  requester_ruc?: string;
}

/** `validityTokenResponse`: the token (a secret; reuse it until `expires_in` seconds run out) or no verdict. */
export type TokenOutcome =
  | { outcome: 'token'; access_token: string; token_type: string | null; expires_in: number | null }
  | ValidityNoVerdict;

/** No answer from the token or the query call (any HTTP status but the documented ones, 401 included, or a timeout). */
export interface ValidityNoVerdict {
  outcome: 'no_verdict';
  stage: 'token' | 'query';
  reason: 'http_status' | 'timeout';
  http_status: number | null;
  message: string;
}

/**
 * `validity`: SUNAT's `estadoCp`. `not_reported` (0) does not prove the voucher never arrived; never
 * issue the operation with another number because of it. `printed_*` (3, 4) apply to printed vouchers.
 * `null` for a code outside the manual.
 */
export type ValidityOutcome =
  | {
      outcome: 'validity';
      state_code: string | null;
      state: 'not_reported' | 'accepted' | 'voided' | 'printed_authorized' | 'printed_not_authorized' | null;
      taxpayer_state_code: string | null;
      domicile_condition_code: string | null;
      /** Where the data sent differs from SUNAT's, as SUNAT words it. */
      observations: string[];
      message: string;
    }
  | { outcome: 'query_error'; http_status: number; error_code: string | null; message: string }
  | ValidityNoVerdict;

/**
 * GRE (despatch advice, 09) by SUNAT's REST API. SUNAT offers no test environment for GRE: nothing
 * here has been accepted by SUNAT. Which call an outcome comes from.
 */
export type GreStage = 'token' | 'send' | 'ticket';

/** A GRE call as `fetch(url, options)` takes it. The token body carries the secret and the SOL password; the others carry the token in `Authorization`: never log them. */
export interface GreRequest {
  method: 'POST' | 'GET';
  url: string;
  headers: Record<string, string>;
  /** Empty for the ticket query (a GET): leave it out of `fetch`, which refuses a body on a GET. */
  body: string;
}

/** `greSendRequest`: the POST plus what to keep with the ticket. */
export interface GreSendRequest extends GreRequest {
  method: 'POST';
  /** `<name>.zip`. */
  zip_name: string;
  /** SHA-256 of the zip, lowercase hex: the `hashZip` sent. */
  hash_zip: string;
}

/** Input of `greSendRequest`: the signed guide. */
export type GreSendInput = { name: string; xml: string } | { name: string; xml_base64: string };

/** SUNAT answered GRE-M's error body (annex I, or 422 with the validation `errors`). */
export interface GreApiError {
  outcome: 'api_error';
  stage: GreStage;
  http_status: number;
  /** Annex I's name for the status, `null` for one outside it. */
  general_error: 'bad_request' | 'unauthorized' | 'forbidden' | 'not_found' | 'not_allowed' | 'not_acceptable' | 'unsupported_media_type' | 'unprocessable_entity' | 'internal_server_error' | 'service_unavailable' | null;
  code: string | null;
  message: string;
  errors: { code: string | null; description: string }[];
  /** The ticket asked, for the ticket stage. */
  ticket?: string;
}

/** No answer, or a body that is not GRE-M's, for the token or the ticket call: ask again. */
export interface GreNoVerdict {
  outcome: 'no_verdict';
  stage: 'token' | 'ticket';
  reason: 'http_status' | 'timeout';
  http_status: number | null;
  message: string;
  ticket?: string;
}

/** `greTokenResponse`: the token (a secret) and when it expires, from the clock you passed. */
export type GreTokenOutcome =
  | { outcome: 'token'; access_token: string; token_type: string | null; expires_in: number | null; expires_at: number | null }
  | GreApiError
  | GreNoVerdict;

/** `greSendResponse`: the ticket, the only handle on the outcome (store it first), or the refusal. */
export type GreSendOutcome = { outcome: 'ticket'; ticket: string; received_at: string | null } | GreApiError;

/**
 * `greTicketResponse`: `state` is `processed` (0), `in_process` (98: ask again with the same ticket,
 * never send again) or `processed_with_errors` (99); `null` for another code.
 */
export type GreTicketOutcome =
  | {
      outcome: 'ticket_status';
      ticket: string;
      status_code: string;
      state: 'processed' | 'in_process' | 'processed_with_errors' | null;
      cdr_generated: boolean;
      /** `numError` and `desError`, with the class of the code. */
      error?: { code: string | null; description: string; class: Cdr['class'] };
      cdr?: Cdr;
      cdr_zip_base64?: string;
    }
  | GreApiError
  | GreNoVerdict;

export interface Sdk {
  /**
   * Renders the unsigned XML of a document (`docs/input-model.md` of the repository).
   * @returns JSON text of a {@link BuildOutput}.
   * @throws Error whose message says which field is wrong.
   */
  build(documentJson: string): string;
  /**
   * Signs an unsigned XML with an unencrypted RSA key.
   * @param signatureId defaults to `signatureKG`.
   * @returns JSON text of a {@link SignOutput}.
   * @throws Error if the XML, the certificate or the key cannot be read.
   */
  sign(xml: string, certPem: string, keyPem: string, signatureId?: string): string;
  /**
   * Zips a signed XML under its name. Input `{"name", "xml" | "xml_base64"}`.
   * @returns JSON text of a {@link ZipOutput}.
   */
  zip(inputJson: string): string;
  /**
   * Reads a CDR. Input `{"cdr_base64"}` (zip or XML).
   * @returns JSON text of a {@link Cdr}.
   */
  cdr(inputJson: string): string;
  /**
   * The POST for an exchange, with the CLI's input for that command and the SOL user (RUC + user).
   * This package does no network I/O: pass the result to `fetch` yourself.
   * @returns JSON text of a {@link SoapRequest}.
   */
  soapRequest(exchange: Exchange, inputJson: string, user: string, password: string): string;
  /**
   * The outcome for SUNAT's HTTP status and body, with the same input given to `soapRequest`:
   * the CLI's output for the same answer.
   * @returns JSON text of a {@link SendOutcome}, {@link SummaryOutcome}, {@link TicketOutcome} or {@link StatusOutcome}.
   * @throws Error for `send` or `summary` without an answer: the document may have arrived.
   */
  soapResponse(exchange: Exchange, inputJson: string, httpStatus: number, body: Uint8Array): string;
  /**
   * The outcome when no answer came within `seconds`.
   * @returns JSON text of a {@link NoVerdictOutcome} for `ticket` and `status`.
   * @throws Error for `send` and `summary`: the outcome is unknown and the document may have arrived.
   */
  soapTimeout(exchange: Exchange, inputJson: string, seconds: number): string;
  /**
   * Consulta Integrada, step 1: the OAuth token request (`client_credentials`, «Credenciales de API SUNAT»).
   * @param base replaces SUNAT's token base `https://api-seguridad.sunat.gob.pe/v1/clientesextranet`.
   * @returns JSON text of a {@link RestRequest}.
   */
  validityTokenRequest(clientId: string, clientSecret: string, base?: string): string;
  /** @returns JSON text of a {@link TokenOutcome}. */
  validityTokenResponse(httpStatus: number, body: Uint8Array): string;
  /**
   * Step 2: the query, with the token. Input: {@link ValidityInput} as JSON text.
   * @param base replaces SUNAT's base `https://api.sunat.gob.pe/v1/contribuyente/contribuyentes`.
   * @returns JSON text of a {@link RestRequest}.
   * @throws Error when the input is not one SUNAT would take.
   */
  validityRequest(inputJson: string, accessToken: string, base?: string): string;
  /** @returns JSON text of a {@link ValidityOutcome}, the CLI's `validity` output for the same answer. */
  validityResponse(httpStatus: number, body: Uint8Array): string;
  /** No answer within `seconds`. @returns JSON text of a {@link ValidityNoVerdict}. */
  validityTimeout(stage: 'token' | 'query', seconds: number): string;
  /**
   * GRE, step 1: the OAuth token request (`grant_type=password`, the GRE pair of «Credenciales de API SUNAT»).
   * @param user the RUC followed by the SOL user.
   * @param tokenBase no default (GRE-M: `https://api-seguridad.sunat.gob.pe/v1/clientessol`).
   * @returns JSON text of a {@link GreRequest}.
   */
  greTokenRequest(clientId: string, clientSecret: string, user: string, password: string, tokenBase: string): string;
  /**
   * @param receivedAt your clock when the answer arrived, in whole Unix seconds (`Math.floor(Date.now() / 1000)`).
   * @returns JSON text of a {@link GreTokenOutcome}.
   */
  greTokenResponse(httpStatus: number, body: Uint8Array, receivedAt: number): string;
  /** Whether a `token` outcome (JSON text) is still usable at `now` (Unix seconds): renewed ten minutes before `expires_at`. */
  greTokenUsable(tokenJson: string, now: number): boolean;
  /**
   * Step 2: the send of the signed guide ({@link GreSendInput} as JSON text), zipped, with its `hashZip`.
   * @param apiBase no default (`https://api-cpe.sunat.gob.pe/v1`).
   * @returns JSON text of a {@link GreSendRequest}.
   * @throws Error when `name` is not `RUC-09-SERIE-NUMERO`.
   */
  greSendRequest(inputJson: string, accessToken: string, apiBase: string): string;
  /**
   * @returns JSON text of a {@link GreSendOutcome}.
   * @throws Error when the answer is not GRE-M's: the guide may have arrived, so send the same file again.
   */
  greSendResponse(httpStatus: number, body: Uint8Array): string;
  /** Step 3: the ticket query, a GET. @returns JSON text of a {@link GreRequest}. */
  greTicketRequest(ticket: string, accessToken: string, apiBase: string): string;
  /** @returns JSON text of a {@link GreTicketOutcome}, with the CDR read when SUNAT attaches it. */
  greTicketResponse(httpStatus: number, body: Uint8Array, ticket: string): string;
  /**
   * No answer within `seconds`.
   * @returns JSON text of a {@link GreNoVerdict} for `token`, and for `ticket` (pass the ticket).
   * @throws Error for `send`: the guide may have arrived.
   */
  greTimeout(stage: GreStage, seconds: number, ticket?: string): string;
}

/** Where to read the .wasm from when the default location does not apply (e.g. Cloudflare Workers). */
export type WasmSource = WebAssembly.Module | BufferSource | URL | Response | Promise<Response> | string;

/** Instantiates the WebAssembly module once and returns the SDK; later calls return the same SDK. */
export function load(source?: WasmSource): Promise<Sdk>;
