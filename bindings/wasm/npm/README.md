# @kembec/facturacion-electronica-sunat-sdk

Arma y firma, en memoria, el XML de los comprobantes electrónicos de la SUNAT (Perú): facturas (01), boletas (03),
notas de crédito (07) y notas de débito (08) en UBL 2.1, y resúmenes diarios (RC) y comunicaciones de baja (RA) en
UBL 2.0. Es para sistemas en JavaScript que emiten comprobantes: corre en Node 18 o posterior y en el navegador con la
misma API, y usa el mismo núcleo en Rust que la CLI y el cliente PHP del
[repositorio](https://github.com/Kembec/facturacion-electronica-sunat-sdk).

**Estado:** versión 0.1.0-beta.2, todavía no publicada en npm; mientras tanto, el paquete se arma desde el repositorio
como explica [su README](https://github.com/Kembec/facturacion-electronica-sunat-sdk#javascript-webassembly). En cada
cambio, el CI instala el paquete empaquetado en un proyecto limpio y exige la firma exacta de los 18 vectores de prueba
en Node 18, Node 22 y Chrome. Son las mismas firmas que la beta de la SUNAT aceptó el 16/10/2026, enviadas por la CLI.
Guías y ejemplos: https://fe.kembec.com

```js
import { load } from '@kembec/facturacion-electronica-sunat-sdk';

const sdk = await load();
const { file_name, xml } = JSON.parse(sdk.build(documentJson));
const signed = JSON.parse(sdk.sign(xml, certPem, keyPem)); // { xml, digest_value, signature_value }
```

- `build(documentJson)`: el documento con la forma del
  [modelo de entrada](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/input-model.md).
- `sign(xml, certPem, keyPem, signatureId?)`: la llave PEM tiene que ser RSA sin cifrar;
  `signatureId` es `signatureKG` si no se indica.
- Las dos devuelven texto JSON y lanzan un `Error` con el motivo cuando la entrada no sirve.
- No hay entrada ni salida: el certificado y la llave los pasa quien llama. El envío a la SUNAT está en la CLI y en
  el cliente PHP del mismo repositorio.

## Dónde se carga el WebAssembly

En Node se lee de disco. En el navegador y con bundlers (Vite, webpack 5) se pide junto a `web.js`.
Donde eso no aplique —por ejemplo, Cloudflare Workers— se pasa el módulo a `load`:

```js
import wasm from '@kembec/facturacion-electronica-sunat-sdk/wasm';
const sdk = await load(wasm);
```

`load` acepta un `WebAssembly.Module`, los bytes, una URL o una `Response`. El CI prueba la carga por defecto en Node y
en Chrome, y el paso del módulo compilado en Node; no en Workers ni con bundlers.

En Node 18, donde `globalThis.crypto` no existe sin una opción de arranque, `load` lo define con
`webcrypto` de `node:crypto`: la firma lo necesita. Si ya existe, no lo toca.

## Licencia

MIT, © 2026 Manuel Benancio: ver
[`LICENSE`](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/LICENSE). Código, pruebas y errores:
<https://github.com/Kembec/facturacion-electronica-sunat-sdk>.
