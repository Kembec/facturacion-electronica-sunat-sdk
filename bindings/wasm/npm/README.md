# SDK de facturación electrónica SUNAT (Perú) para JavaScript y TypeScript

Un SDK para emitir comprobantes electrónicos ante la SUNAT desde **JavaScript y TypeScript**, con la misma API en Node
18 o posterior y en el navegador. Con los datos de una factura (01), una boleta (03), una nota de crédito (07) o de
débito (08), un resumen diario de boletas (RC) o una comunicación de baja (RA):

- arma el archivo XML que pide la SUNAT, lo firma con tu certificado digital y lo comprime;
- te da la petición a la SUNAT lista para tu propio `fetch`, y lee la respuesta, que dice si lo aceptó;
- arma la consulta del estado de un comprobante ya enviado en la Consulta Integrada de la SUNAT (`validity`).

Es la versión para JavaScript y TypeScript del
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
base=https://raw.githubusercontent.com/Kembec/facturacion-electronica-sunat-sdk/v0.1.0-beta.4
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

Si imprime `20123456789-01-F001-1 +te61a2ub5MlTFf8tKTPNGWEXKI=` (el nombre del archivo y la huella de la firma), el
paquete funciona: es la misma salida que dan la CLI, PHP, Python y Rust. Tus propios documentos siguen la forma del
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

## Consultar el estado de un comprobante (Consulta Integrada)

`validityRequest` pregunta a la Consulta Integrada de la SUNAT si una factura, boleta o nota (01, 03, 07, 08) existe y
en qué estado está. Usa las «Credenciales de API SUNAT» que se crean en SOL para esta consulta: no el usuario SOL ni el
par de la GRE. Son dos llamadas, y las dos van con tu `fetch`:

```js
const read = async (res) => new Uint8Array(await res.arrayBuffer());
const tokenCall = JSON.parse(sdk.validityTokenRequest(clientId, clientSecret));
const tokenRes = await fetch(tokenCall.url, tokenCall);
const token = JSON.parse(sdk.validityTokenResponse(tokenRes.status, await read(tokenRes)));
if (token.outcome !== 'token') throw new Error(token.message); // no_verdict: vuelve a intentar
const input = { ruc: '20123456789', document_type: '01', series: 'F001', number: 123, issue_date: '2026-10-16', total: '118.00' };
const queryCall = JSON.parse(sdk.validityRequest(JSON.stringify(input), token.access_token));
const queryRes = await fetch(queryCall.url, queryCall);
const check = JSON.parse(sdk.validityResponse(queryRes.status, await read(queryRes)));
// { outcome: 'validity', state: 'accepted' | 'voided' | 'not_reported' | ..., observations } · query_error · no_verdict
```

- **No hay ambiente de pruebas**: por defecto la consulta va a producción; el último argumento opcional de
  `validityTokenRequest` y `validityRequest` reemplaza la URL base. El token es un secreto: no lo guardes en logs.
- Ningún resultado es motivo para emitir la operación con otro número: `not_reported` no prueba que el comprobante no
  llegó (el ticket de un resumen puede seguir en proceso), así que antes se concilia con `ticket` o `status`.
- Sin respuesta (un 401 incluido) o al acabarse la espera (`validityTimeout(stage, seconds)`), el resultado es
  `{ outcome: 'no_verdict', stage, ... }`: vuelve a consultar con los mismos datos.

## Guía de remisión remitente (09) por la API REST

Estas funciones están en la rama `main` y todavía no en un release: la 0.1.0-beta.4 de npm no las trae.

La guía no va por SOAP: un token con la clave SOL, el envío del zip y la consulta del ticket. El paquete arma cada
llamada (con su `url`) y lee cada respuesta; el `fetch` es tuyo, como con `soapRequest`.

```js
const read = async (res) => new Uint8Array(await res.arrayBuffer());
// 1. Token: el par de GRE de «Credenciales de API SUNAT» y el usuario SOL (RUC + usuario).
const tokenCall = JSON.parse(sdk.greTokenRequest(clientId, clientSecret, solUser, solPassword, tokenBase));
const tokenRes = await fetch(tokenCall.url, tokenCall);
const token = JSON.parse(sdk.greTokenResponse(tokenRes.status, await read(tokenRes), Math.floor(Date.now() / 1000)));
if (token.outcome !== 'token') throw new Error(token.message); // api_error o no_verdict: no se envió nada
// 2. Envío de la guía firmada: guarda el ticket antes que nada, es lo único que lleva al resultado.
const sendCall = JSON.parse(sdk.greSendRequest(JSON.stringify({ name: file_name, xml: signed.xml }), token.access_token, apiBase));
const sendRes = await fetch(sendCall.url, sendCall);
const sent = JSON.parse(sdk.greSendResponse(sendRes.status, await read(sendRes))); // { outcome: 'ticket', ticket } o api_error
// 3. Consulta del ticket: 0 processed (con la CDR), 98 in_process (vuelve a consultar), 99 processed_with_errors.
const ticketCall = JSON.parse(sdk.greTicketRequest(sent.ticket, token.access_token, apiBase));
const ticketRes = await fetch(ticketCall.url, { method: 'GET', headers: ticketCall.headers }); // fetch no acepta body en un GET
const status = JSON.parse(sdk.greTicketResponse(ticketRes.status, await read(ticketRes), sent.ticket));
```

**La SUNAT no tiene ambiente de pruebas para la GRE**, y ninguna guía de este SDK figura como aceptada por la SUNAT. Las
URL base no tienen valor por defecto (las del manual: `https://api-seguridad.sunat.gob.pe/v1/clientessol` para el token y
`https://api-cpe.sunat.gob.pe/v1` para la API).

- `greTokenResponse` pide la hora de llegada en segundos Unix y devuelve `expires_at`. `greTokenUsable(tokenJson, now)`
  dice si el token sirve todavía; se renueva diez minutos antes de vencer.
- El token y el cuerpo del pedido de token son secretos: no los guardes en logs.
- Si el envío no recibe respuesta, o recibe una que no es de la SUNAT, `greSendResponse` y `greTimeout('send', s)` lanzan
  un `Error`: la guía pudo haber llegado, así que se reenvía el mismo archivo, nunca la operación con otro número.
  `greTimeout('token', s)` y `greTimeout('ticket', s, ticket)` dan `{ outcome: 'no_verdict', stage, ... }`.
- Un rechazo de la SUNAT llega como `{ outcome: 'api_error', stage, http_status, general_error, code, message, errors }`.
- Con 98, vuelve a consultar con el mismo ticket; nunca reenvíes la guía por eso.

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
- Si `ticket` o `status` no reciben respuesta, el resultado es `{ outcome: 'no_verdict', reason, http_status, message }`
  (`ticket` agrega el mismo `ticket`). Cuenta como sin respuesta un estado HTTP distinto de 200 y 500, o que se acabe la
  espera. No dice nada del documento: vuelve a consultar. Con `ticket`, nunca reenvíes el resumen: el ticket prueba que
  llegó. Si `status` sigue sin veredicto, reenvía el mismo XML firmado con el mismo número mientras el plazo de envío
  siga abierto, porque la SUNAT lo mide al recibirlo; según sus reglas de validación, si ya lo tenía no lo registra dos
  veces y contesta 1033, 1032 o 0140.
- En `send` y `summary`, ese mismo caso lanza un `Error`, porque el documento pudo haber llegado. Un `summary` sin
  respuesta no deja ticket: revisa los comprobantes con la Consulta Integrada y reenvía el mismo archivo con el mismo
  nombre.
- `validityTokenRequest`, `validityTokenResponse`, `validityRequest`, `validityResponse` y `validityTimeout`: la
  Consulta Integrada, [arriba](#consultar-el-estado-de-un-comprobante-consulta-integrada).
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

## Límites y pruebas

- El envío desde el navegador depende de que la SUNAT acepte peticiones de otro origen (CORS), y eso no se ha probado.
- La carga con el módulo ya compilado se prueba en Node; en Cloudflare Workers y con bundlers, no.
- Lo que aceptó la SUNAT y cómo se comprueba cada versión está en
  [Cómo se comprueba](https://github.com/Kembec/facturacion-electronica-sunat-sdk#cómo-se-comprueba): la beta de la
  SUNAT aceptó 36 de los 38 documentos de ejemplo; las 2 guías de remisión no tienen beta. Para este paquete, el CI lo
  instala en un proyecto limpio y exige la firma exacta de los 38 en Node 18, Node 22 y Chrome; el envío se prueba sin
  red, con respuestas escritas a mano (la GRE, con las respuestas grabadas del núcleo).

## Licencia

MIT, © 2026 Manuel Benancio: ver
[`LICENSE`](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/LICENSE). Código, pruebas y errores:
<https://github.com/Kembec/facturacion-electronica-sunat-sdk>.
