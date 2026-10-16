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
}

/** Where to read the .wasm from when the default location does not apply (e.g. Cloudflare Workers). */
export type WasmSource = WebAssembly.Module | BufferSource | URL | Response | Promise<Response> | string;

/** Instantiates the WebAssembly module once and returns the SDK; later calls return the same SDK. */
export function load(source?: WasmSource): Promise<Sdk>;
