// Node loader (`node` condition): reads the .wasm from disk and instantiates it synchronously,
// because Node 18-20 `fetch` does not read file: URLs, which is what the web loader would do.
import { readFileSync } from 'node:fs';
import { webcrypto } from 'node:crypto';
import { pathToFileURL } from 'node:url';
import {
  initSync, build, sign, zip, cdr, soapRequest, soapResponse, soapTimeout, diagnose,
  validityTokenRequest, validityTokenResponse, validityRequest, validityResponse, validityTimeout,
  greTokenRequest, greTokenResponse, greTokenUsable, greSendRequest, greSendResponse, greTicketRequest, greTicketResponse, greTimeout,
} from './wasm/sunat_sdk_wasm.js';

const DEFAULT_WASM = new URL('./wasm/sunat_sdk_wasm_bg.wasm', import.meta.url);

let ready;

// Accepts what the web loader accepts: a compiled module or bytes as they are, a Response by its
// body, a file: URL or a path from disk, and any other URL through fetch.
async function moduleSource(source) {
  if (source === undefined) return readFileSync(DEFAULT_WASM);
  const value = await source;
  if (value instanceof WebAssembly.Module || value instanceof ArrayBuffer || ArrayBuffer.isView(value)) return value;
  if (typeof Response !== 'undefined' && value instanceof Response) return new Uint8Array(await value.arrayBuffer());
  const url = value instanceof URL ? value : new URL(String(value), pathToFileURL(`${process.cwd()}/`));
  if (url.protocol === 'file:') return readFileSync(url);
  const response = await fetch(url);
  if (!response.ok) throw new Error(`${url}: HTTP ${response.status}`);
  return new Uint8Array(await response.arrayBuffer());
}

export function load(source) {
  ready ??= moduleSource(source)
    .then((module) => {
      // RSA blinding draws randomness through getrandom's `js` backend, which needs
      // globalThis.crypto; Node 18 only exposes it behind a flag, and its CommonJS fallback is
      // unreachable from an ES module. Set it only when absent.
      if (!globalThis.crypto?.getRandomValues) {
        Object.defineProperty(globalThis, 'crypto', { value: webcrypto, configurable: true, writable: true });
      }
      initSync({ module });
      return Object.freeze({
        build, sign, zip, cdr, soapRequest, soapResponse, soapTimeout, diagnose,
        validityTokenRequest, validityTokenResponse, validityRequest, validityResponse, validityTimeout,
        greTokenRequest, greTokenResponse, greTokenUsable, greSendRequest, greSendResponse, greTicketRequest, greTicketResponse, greTimeout,
      });
    })
    .catch((error) => {
      ready = undefined;
      throw error;
    });
  return ready;
}
