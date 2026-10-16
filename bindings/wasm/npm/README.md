# @kembec/facturacion-electronica-sunat-sdk

Arma el XML UBL 2.1 de facturas (01), boletas (03), notas de crédito (07) y notas de débito (08) de
la SUNAT (Perú) y lo firma, en memoria, con el mismo núcleo en Rust que el CLI. Corre en Node 18 o
posterior y en el navegador con la misma API.

```js
import { load } from '@kembec/facturacion-electronica-sunat-sdk';

const sdk = await load();
const { file_name, xml } = JSON.parse(sdk.build(documentJson));
const signed = JSON.parse(sdk.sign(xml, certPem, keyPem)); // { xml, digest_value, signature_value }
```

- `build(documentJson)`: el documento con la forma de `docs/input-model.md` del repositorio.
- `sign(xml, certPem, keyPem, signatureId?)`: la llave PEM tiene que ser RSA sin cifrar;
  `signatureId` es `signatureKG` si no se indica.
- Las dos devuelven texto JSON y lanzan un `Error` con el motivo cuando la entrada no sirve.
- No hay entrada ni salida: el certificado y la llave los pasa quien llama. El envío a la SUNAT está
  en el CLI del mismo repositorio.

## Dónde se carga el WebAssembly

En Node se lee de disco. En el navegador y con bundlers (Vite, webpack 5) se pide junto a `web.js`.
Donde eso no aplique —por ejemplo, Cloudflare Workers— se pasa el módulo a `load`:

```js
import wasm from '@kembec/facturacion-electronica-sunat-sdk/wasm';
const sdk = await load(wasm);
```

`load` acepta un `WebAssembly.Module`, los bytes, una URL o una `Response`.

En Node 18, donde `globalThis.crypto` no existe sin una opción de arranque, `load` lo define con
`webcrypto` de `node:crypto`: la firma lo necesita. Si ya existe, no lo toca.

Licencia MIT. Código y pruebas: <https://github.com/Kembec/facturacion-electronica-sunat-sdk>.
