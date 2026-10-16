# SDK de facturación electrónica SUNAT (Perú) para JavaScript y TypeScript

Un SDK para emitir comprobantes electrónicos ante la SUNAT desde **JavaScript y TypeScript**, con la misma API en Node
18 o posterior y en el navegador. Con los datos de una factura (01), una boleta (03), una nota de crédito (07) o de
débito (08), un resumen diario de boletas (RC) o una comunicación de baja (RA), arma el archivo XML que pide la SUNAT,
lo firma con tu certificado digital y lo comprime. El envío lo haces con tu propio `fetch`: el paquete te da la
petición lista y lee la respuesta de la SUNAT, que dice si lo aceptó. Es la versión para JavaScript y TypeScript del
[SDK](https://github.com/Kembec/facturacion-electronica-sunat-sdk), que también funciona en PHP, Python, Rust y la
línea de comandos: los mismos datos dan el mismo comprobante firmado en todos.

SUNAT (Peru) electronic invoicing SDK for JavaScript and TypeScript, in Node and in the browser.

Tu sistema calcula los importes; el paquete los valida y los pone en el XML, pero no calcula impuestos. Guías y
ejemplos: https://fe.kembec.com

## Instalación

```bash
npm install @kembec/facturacion-electronica-sunat-sdk@beta
```

Los tipos de TypeScript vienen en el paquete (`index.d.ts`).

## Tu primer comprobante

Baja a una carpeta vacía la factura de prueba y el certificado de prueba del repositorio. Ese certificado es
autogenerado y no sirve para emitir.

```bash
base=https://raw.githubusercontent.com/Kembec/facturacion-electronica-sunat-sdk/v0.1.0-beta.3
curl -fsSLO "$base/fixtures/01-invoice-taxed-cash.json"
curl -fsSLO "$base/tests/vectors/test-cert/test-cert.pem"
curl -fsSLO "$base/tests/vectors/test-cert/test-key.pem"
```

```js
// primer-comprobante.mjs
import { readFileSync } from 'node:fs';
import { load } from '@kembec/facturacion-electronica-sunat-sdk';

const read = (file) => readFileSync(file, 'utf8');
const sdk = await load();
const { file_name, xml } = JSON.parse(sdk.build(read('01-invoice-taxed-cash.json')));
const signed = JSON.parse(sdk.sign(xml, read('test-cert.pem'), read('test-key.pem')));
console.log(file_name, signed.digest_value); // 20123456789-01-F001-1 +te61a2ub5MlTFf8tKTPNGWEXKI=
```

`+te61a2ub5MlTFf8tKTPNGWEXKI=` es el `DigestValue` del vector
[`01-invoice-taxed-cash`](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/tests/vectors/01-invoice-taxed-cash.signed.xml):
el mismo que dan la CLI, PHP, Python y Rust. Tus documentos llevan la forma del
[modelo de entrada](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/input-model.md).

## Enviar a la SUNAT

El paquete no hace red: `soapRequest` te da el POST ya armado, tú lo envías y `soapResponse` convierte la respuesta en
el mismo JSON que imprime la CLI.

```js
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

## API

Todas las funciones reciben y devuelven texto JSON, y lanzan un `Error` con el motivo cuando la entrada no sirve.

- `build(documentJson)`: el documento con la forma del
  [modelo de entrada](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/input-model.md) →
  `{ file_name, xml }`.
- `sign(xml, certPem, keyPem, signatureId?)`: la llave PEM tiene que ser RSA sin cifrar; `signatureId` es
  `signatureKG` si no se indica → `{ xml, digest_value, signature_value }`.
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
- El certificado, la llave y las credenciales SOL los pasa quien llama: el paquete no los busca en archivos ni en el entorno.

## Dónde se carga el WebAssembly

El paquete es el núcleo en Rust del SDK compilado a WebAssembly, sin binarios ni procesos aparte: arma XML UBL 2.1
(comprobantes) y UBL 2.0 (resúmenes y bajas) y lo firma con XML-DSig.

En Node se lee de disco. En el navegador y con bundlers (Vite, webpack 5) se pide junto a `web.js`.
Donde eso no aplique —por ejemplo, Cloudflare Workers— se pasa el módulo a `load`:

```js
import wasm from '@kembec/facturacion-electronica-sunat-sdk/wasm';
const sdk = await load(wasm);
```

`load` acepta un `WebAssembly.Module`, los bytes, una URL o una `Response`.

En Node 18, donde `globalThis.crypto` no existe sin una opción de arranque, `load` lo define con
`webcrypto` de `node:crypto`: la firma lo necesita. Si ya existe, no lo toca.

## Cómo se comprueba

En cada cambio, el CI instala el paquete empaquetado en un proyecto limpio y exige, en Node 18, Node 22 y Chrome, el
`DigestValue` y el `SignatureValue` exactos de los 38 vectores de prueba del repositorio: las mismas firmas que la beta de la SUNAT aceptó el
16/10/2026, enviadas con la CLI. La carga con el módulo ya compilado se prueba en Node; en Cloudflare Workers y con
bundlers no se ha probado. El envío desde JavaScript se prueba sin red, con respuestas escritas a mano, y desde el
navegador depende de que la SUNAT acepte peticiones de otro origen (CORS), que no se ha probado.

## Licencia

MIT, © 2026 Manuel Benancio: ver
[`LICENSE`](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/LICENSE). Código, pruebas y errores:
<https://github.com/Kembec/facturacion-electronica-sunat-sdk>.
