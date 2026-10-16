# @kembec/facturacion-electronica-sunat-sdk

Arma, firma y envía a la SUNAT (Perú) los comprobantes electrónicos desde JavaScript: facturas (01), boletas (03),
notas de crédito (07) y notas de débito (08) en UBL 2.1, y resúmenes diarios (RC) y comunicaciones de baja (RA) en
UBL 2.0. Es para sistemas en JavaScript que emiten comprobantes: corre en Node 18 o posterior y en el navegador con la
misma API, y usa el mismo núcleo en Rust que la CLI y el cliente PHP del
[repositorio](https://github.com/Kembec/facturacion-electronica-sunat-sdk). El paquete no hace red: arma el POST, tú
lo envías con tu `fetch`, y convierte la respuesta en el mismo JSON que imprime la CLI.

**Estado:** versión 0.1.0-beta.2, todavía no publicada en npm; mientras tanto, el paquete se arma desde el repositorio
como explica [su README](https://github.com/Kembec/facturacion-electronica-sunat-sdk#javascript-webassembly). En cada
cambio, el CI instala el paquete empaquetado en un proyecto limpio y exige la firma exacta de los 20 vectores de prueba
en Node 18, Node 22 y Chrome. Son las mismas firmas que la beta de la SUNAT aceptó el 16/10/2026, enviadas por la CLI.
El envío y las consultas desde JavaScript (`soapRequest`, `soapResponse`, `soapTimeout`) los prueban las pruebas
unitarias y la prueba de Node del CI, sin red; todavía no se ha enviado nada a la SUNAT desde este paquete.
Guías y ejemplos: https://fe.kembec.com

```js
import { load } from '@kembec/facturacion-electronica-sunat-sdk';

const sdk = await load();
const { file_name, xml } = JSON.parse(sdk.build(documentJson));
const signed = JSON.parse(sdk.sign(xml, certPem, keyPem)); // { xml, digest_value, signature_value }

// Una llamada al web service de la SUNAT: el paquete arma el POST y lee la respuesta; el fetch es tuyo.
async function exchange(kind, input, url, seconds = 60) {
  const post = JSON.parse(sdk.soapRequest(kind, input, solUser, solPassword)); // solUser: RUC + usuario SOL
  const signal = AbortSignal.timeout(seconds * 1000);
  try {
    const res = await fetch(url, { ...post, signal });
    return JSON.parse(sdk.soapResponse(kind, input, res.status, new Uint8Array(await res.arrayBuffer())));
  } catch (error) {
    if (!signal.aborted) throw error;
    return JSON.parse(sdk.soapTimeout(kind, input, seconds)); // en send y summary lanza un Error
  }
}

const sent = await exchange('send', JSON.stringify({ name: file_name, xml: signed.xml }), billServiceUrl);
// { outcome: 'cdr', cdr, cdr_zip_base64 } o { outcome: 'fault', code, message, class }
```

- `build(documentJson)`: el documento con la forma del
  [modelo de entrada](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/input-model.md).
- `sign(xml, certPem, keyPem, signatureId?)`: la llave PEM tiene que ser RSA sin cifrar;
  `signatureId` es `signatureKG` si no se indica.
- `zip(inputJson)`: `{ name, xml }` (o `xml_base64`) → `{ file_name, zip_base64 }`.
- `cdr(inputJson)`: `{ cdr_base64 }` (zip o XML) → `{ reference_id, response_code, description, notes, class }`.
- `soapRequest(exchange, inputJson, user, password)`: `exchange` es `send`, `summary`, `ticket` o `status`, e
  `inputJson`, la entrada de ese comando de la CLI (`send` y `summary`, `{ name, xml }`; `ticket`, `{ ticket }`;
  `status`, `{ ruc, document_type, series, number }`). Devuelve `{ method, headers, body }` para
  `fetch(url, opciones)`. El `body` lleva las credenciales SOL: no lo guardes en logs. `send`, `summary` y `ticket` van
  a la URL del billService, y `status`, a la del billConsultService.
- `soapResponse(exchange, inputJson, httpStatus, body)`: con la misma entrada, el estado HTTP y el cuerpo como
  `Uint8Array`, devuelve lo que la CLI imprime para esa respuesta
  ([contrato de la CLI](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/cli.md)). Un error SOAP
  de la SUNAT no lanza: llega como `{ outcome: 'fault', ... }`.
- `soapTimeout(exchange, inputJson, seconds)`: el resultado cuando no llegó respuesta en `seconds` segundos.
- Si `ticket` o `status` no reciben respuesta (un estado HTTP distinto de 200 y 500, o se acabó la espera), el
  resultado es `{ outcome: 'no_verdict', reason, http_status, message }` (`ticket` agrega el mismo `ticket`): no dice
  nada del documento, así que se vuelve a consultar y nunca se reenvía. En `send` y `summary`, ese mismo caso lanza un
  `Error`, porque el documento pudo haber llegado.
- Todas devuelven texto JSON y lanzan un `Error` con el motivo cuando la entrada no sirve. Los tipos de cada resultado
  están en `index.d.ts`.
- El certificado, la llave y las credenciales SOL los pasa quien llama: el paquete no los busca en archivos ni en el entorno.
- Desde el navegador, el envío no se ha probado: depende de que la SUNAT acepte peticiones de otro origen (CORS).

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
