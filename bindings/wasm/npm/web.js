// Default loader (browsers, Deno, bundlers): fetches the .wasm next to this file, or takes the
// source the caller passes (a URL, a Response, the bytes or a compiled WebAssembly.Module).
import init, { build, sign } from './wasm/sunat_sdk_wasm.js';

let ready;

export function load(source) {
  ready ??= init({ module_or_path: source ?? new URL('./wasm/sunat_sdk_wasm_bg.wasm', import.meta.url) })
    .then(() => Object.freeze({ build, sign }))
    .catch((error) => {
      ready = undefined;
      throw error;
    });
  return ready;
}
