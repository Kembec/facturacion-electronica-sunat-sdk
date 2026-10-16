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
}

/** Where to read the .wasm from when the default location does not apply (e.g. Cloudflare Workers). */
export type WasmSource = WebAssembly.Module | BufferSource | URL | Response | Promise<Response> | string;

/** Instantiates the WebAssembly module once and returns the SDK; later calls return the same SDK. */
export function load(source?: WasmSource): Promise<Sdk>;
