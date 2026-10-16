# facturacion-electronica-sunat-sdk

Facturación electrónica de la SUNAT (Perú) desde PHP, JavaScript o la línea de comandos, con un mismo núcleo en Rust:
los tres producen el mismo XML y la misma firma. Es para quien programa el sistema de facturación de un emisor
electrónico: tu sistema calcula los importes y decide qué emitir; el SDK arma el XML, lo firma, lo envía y lee la
respuesta de la SUNAT.

**Estado:** versión 0.1.0-beta.2. Genera y firma facturas, boletas, notas de crédito, notas de débito, resúmenes
diarios y comunicaciones de baja, y en cada cambio el CI exige, en los tres, la firma exacta de 20 vectores de prueba.
La beta de la SUNAT aceptó los 14 vectores de comprobantes y procesó los 6 de resúmenes y bajas (16/10/2026). Los
paquetes de Composer y npm aún no están publicados.

Guías y los 12 ejemplos, con su entrada, su XML y su firma: **https://fe.kembec.com**

## Qué hace hoy

| Capacidad | Estado | Cómo se comprueba |
|---|---|---|
| Factura (01), boleta (03), nota de crédito (07) y nota de débito (08): XML UBL 2.1 y firma | Comprobado en CI | En cada cambio, el CI genera y firma los 14 casos de comprobantes de `fixtures/` y exige el XML de `tests/vectors/` en forma canónica, y exactamente su `DigestValue` y su `SignatureValue`. Lo hace por la CLI, por PHP y por el paquete npm en Node 18, Node 22 y Chrome. Un verificador en Python, que no comparte código con el núcleo, acepta las firmas. |
| Descuento por línea que reduce la base del IGV (catálogo 53, código 00), en factura y boleta | Comprobado en CI y en la beta de la SUNAT | Dos de esos 14 casos, `01-invoice-line-discount` y `03-receipt-line-discount`, lo llevan; cada descuento se escribe como `cac:AllowanceCharge` con `ChargeIndicator` en `false`. El 16/10/2026 la beta aceptó los dos (F001-7 y B001-4): código 0, sin observaciones. |
| ZIP y lectura de la CDR | Comprobado en CI, sin red | Pruebas con CDR sintéticas, escritas según los ejemplos del manual de la SUNAT. |
| Envío (`send`) | Comprobado en la beta de la SUNAT | El 16/10/2026 la beta aceptó los 14 vectores firmados, enviados por la CLI tal como están en `tests/vectors/`: código 0, sin observaciones. |
| Consulta de la CDR (`status`) | Implementado, sin probar contra la SUNAT | La SUNAT ofrece `getStatusCdr` solo en producción. El sobre sigue el ejemplo del manual. |
| Resumen diario (RC) y comunicación de baja (RA): XML UBL 2.0, firma, envío y consulta por ticket | Comprobado en CI; envío comprobado en la beta | El CI genera y firma los 6 casos `rc-` y `ra-` de `fixtures/` con las mismas exigencias que los 14 comprobantes. El 16/10/2026 la beta procesó los 6 vectores firmados, enviados con `summary` y seguidos con `ticket`: código 0 en todos. Desde PHP se envían y consultan con la CLI. |
| Envío y consultas desde JavaScript (`soapRequest`, `soapResponse`, `soapTimeout`) | Comprobado en CI, sin red | El paquete npm arma el POST de `send`, `summary`, `ticket` y `status` y lee la respuesta con el mismo código de la CLI. Lo prueban las pruebas unitarias del binding y la prueba de Node del CI, sin red y con respuestas escritas a mano. Todavía no se ha enviado nada a la SUNAT desde JavaScript. |
| Guía de remisión remitente (09): XML UBL 2.1 y firma | Comprobado en CI, sin la SUNAT; el envío no está incluido | En cada cambio, el CI genera y firma los 2 casos `09-` de `fixtures/` por el núcleo, la CLI, PHP, el paquete npm y Python, y exige exactamente el `DigestValue` y el `SignatureValue` de `tests/vectors/`; el verificador independiente acepta las firmas. Esos vectores los escribió el propio SDK: detectan regresiones, no prueban compatibilidad. Además, el XML firmado valida contra el esquema OASIS UBL 2.1 de `DespatchAdvice`, y hay una prueba por cada código de regla de la SUNAT que se puede comprobar sin red. La SUNAT no tiene ambiente de pruebas para guías y las recibe solo por su servicio REST, que el SDK no cubre. Rechaza los motivos de traslado 08, 09 y 18 (importación, exportación y emisor itinerante) y la guía del transportista (31). |
| Retención y percepción | No incluidos | — |

Un tipo de documento se da por incorporado cuando cumple tres criterios: su XML coincide con los vectores, su firma da
los mismos valores y la acepta un verificador independiente, y la beta de la SUNAT lo acepta. Los cuatro tipos de la
primera fila cumplen los tres. La guía de remisión remitente no puede cumplir el tercero, porque la SUNAT no tiene
ambiente de pruebas para guías: en su lugar se exigen el esquema OASIS y las pruebas por código de regla. El detalle
está en el [modelo de entrada](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/input-model.md),
sección «Despatch advice (09)».

**Solo IGV, y sin cálculo de impuestos.** Los importes los calcula tu sistema y llegan como texto (`"118.00"`): el SDK
los valida y los escribe. Rechaza con un error explícito ISC, ICBPER, IVAP y otros tributos, descuentos y cargos
globales, cargos por línea, descuentos por línea con un código distinto de 00 o en notas de crédito y débito,
anticipos, detracción, percepción y referencias a guías. En los resúmenes rechaza además percepción,
ICBPER, ISC e IVAP en las líneas, y los resúmenes de reversión (RR).

**Con descuento por línea, `unit_value` es el valor antes del descuento.** El SDK valida el formato de cada descuento
(`factor`, `amount`, `base_amount`), pero la aritmética es de tu sistema: `line_value` debe ser `unit_value` ×
`quantity` menos los descuentos. En la beta, el mismo caso con un `unit_value` ya neto del descuento dio el rechazo 3271
en la factura y la observación 4288 en la boleta. El detalle está en el
[modelo de entrada](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/input-model.md).

## Instalación

> **Composer y npm, todavía no.** Los paquetes están preparados, pero no están en Packagist ni en npm: hoy
> `composer require` y `npm install` fallan. `vendor/bin/sunat-sdk-install` descarga el binario del release de su misma
> versión, 0.1.0-beta.2, que todavía no está publicado. Cada sección muestra cómo se instalará y cómo usarlo hoy desde
> este repositorio.

### Línea de comandos

Hoy se compila con Rust; el CI usa la versión 1.94.1.

```bash
git clone https://github.com/Kembec/facturacion-electronica-sunat-sdk.git
cd facturacion-electronica-sunat-sdk
cargo build -p sunat-sdk-cli --release --locked
export PATH="$PWD/target/release:$PATH"
```

Comprueba en tu máquina lo mismo que el CI: firma un caso de prueba con el certificado del repositorio y compara el
`DigestValue` con el de su vector.

```bash
sunat-sdk build < fixtures/01-invoice-taxed-cash.json \
  | sunat-sdk sign --cert tests/vectors/test-cert/test-cert.pem --key tests/vectors/test-cert/test-key.pem \
  > firmada.json
grep -o '"digest_value":"[^"]*"' firmada.json
grep -o '<ds:DigestValue>[^<]*' tests/vectors/01-invoice-taxed-cash.signed.xml
# los dos dan +te61a2ub5MlTFf8tKTPNGWEXKI=
```

Ese certificado es autogenerado y no sirve para emitir. Cada comando lee un JSON por la entrada estándar y escribe un
JSON por la salida; `send`, `summary`, `ticket` y `status` toman la URL y las credenciales SOL de opciones o del entorno. El contrato
completo está en [`docs/cli.md`](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/cli.md), y el
modelo de entrada, en
[`docs/input-model.md`](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/input-model.md).

Cada [release](https://github.com/Kembec/facturacion-electronica-sunat-sdk/releases) trae el binario estático para
Linux x86_64 y su SHA-256. El único publicado, v0.1.0-beta.1, es anterior a `summary`, `ticket` y a la generación de
resúmenes y bajas.

### PHP

PHP 7.4 o posterior, con la extensión `json` (el instalador del binario usa además `openssl`). Cada llamada ejecuta el
binario `sunat-sdk` una vez.

Cuando el paquete esté publicado:

```bash
composer require kembec/facturacion-electronica-sunat-sdk
vendor/bin/sunat-sdk-install
```

`sunat-sdk-install` descarga del release de la misma versión el binario de Linux x86_64, lo deja ejecutable solo si su
SHA-256 coincide e imprime `SUNAT_SDK_BIN=<ruta>`: esa ruta es el primer argumento de `SunatSdk`. Composer no lo ejecuta
por su cuenta. En otra plataforma, compila el binario como en la sección anterior.

Hoy, con este repositorio clonado al lado de tu proyecto y la CLI ya compilada, declara el clon como repositorio `path`
en el `composer.json` de tu proyecto y corre `composer install`:

```json
{
    "repositories": [
        { "type": "path", "url": "../facturacion-electronica-sunat-sdk" }
    ],
    "require": {
        "kembec/facturacion-electronica-sunat-sdk": "*@dev"
    }
}
```

```php
use Kembec\SunatSdk\SigningKey;
use Kembec\SunatSdk\SunatSdk;

// La ruta que imprime sunat-sdk-install (hoy, target/release/sunat-sdk del clon)
// y el tiempo máximo por llamada, en segundos
$sdk = new SunatSdk('/ruta/a/sunat-sdk', 120.0);

$built  = $sdk->build($documento); // array, objeto o JSON con la forma de docs/input-model.md
$signed = $sdk->sign($built->getXml(), new SigningKey('/ruta/cert.pem', '/ruta/clave.pem'));

echo $built->getFileName();      // por ejemplo, 20123456789-01-F001-1
echo $signed->getDigestValue();
```

Los errores llegan como excepciones que heredan de `SunatSdkException`. Un error SOAP de la SUNAT no es una excepción,
sino un resultado (`isFault()`). `send`, `status`, `zip` y `cdr`, y la tabla de excepciones, están en el
[README del binding](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/bindings/php/README.md).
Los resúmenes y las bajas se arman y firman con `build` y `sign`; para enviarlos y consultar su ticket, usa la CLI.

### JavaScript (WebAssembly)

Lo mismo que la CLI, con la misma API en Node 18 o posterior y en el navegador: `build`, `sign`, `zip` y `cdr` en
memoria, y el envío y las consultas a la SUNAT con tu propio `fetch`. El paquete no hace red. `soapRequest` te da el
POST ya armado, tú lo envías, y `soapResponse` convierte la respuesta en el mismo JSON que imprime la CLI. No necesita
un binario ni un proceso aparte, así que puede ir donde la CLI no llega y solo hay `fetch`, como un Cloudflare Worker;
ahí todavía no se ha probado.

Cuando el paquete esté publicado:

```bash
npm install @kembec/facturacion-electronica-sunat-sdk
```

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

Todas las funciones devuelven texto JSON y lanzan un `Error` con el motivo cuando la entrada no sirve. Cada llamada a
la SUNAT recibe la misma entrada que su comando de la CLI (`send` y `summary`, `{ name, xml }`; `ticket`, `{ ticket }`;
`status`, `{ ruc, document_type, series, number }`) y devuelve lo mismo que imprime
([`docs/cli.md`](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/cli.md)). `send`,
`summary` y `ticket` van a la URL del billService, y `status`, a la del billConsultService.

- Si `ticket` o `status` no reciben respuesta (un estado HTTP distinto de 200 y 500, o se acabó la espera), el
  resultado es `{ outcome: 'no_verdict', ... }`: no dice nada del documento, así que se vuelve a consultar y nunca se
  reenvía.
- En `send` y `summary`, ese mismo caso lanza un `Error`, porque el documento pudo haber llegado.
- Un error SOAP de la SUNAT no lanza: llega como `{ outcome: 'fault', ... }`.
- Desde el navegador, el envío no se ha probado: depende de que la SUNAT acepte peticiones de otro origen (CORS).

`load` también acepta el módulo WebAssembly ya compilado, que es lo que piden entornos como Cloudflare Workers. El CI
lo prueba así en Node, pero no en Workers ni con bundlers; esos casos están descritos en el
[README del paquete](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/bindings/wasm/npm/README.md).

Hoy, el paquete se arma desde este repositorio con los mismos pasos del CI. Necesitas `rustup` y Node 18 o posterior:

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129 --locked
bindings/wasm/scripts/build.sh
(cd bindings/wasm/npm && npm pack)
```

Y en tu proyecto:

```bash
npm install ../facturacion-electronica-sunat-sdk/bindings/wasm/npm/kembec-facturacion-electronica-sunat-sdk-0.1.0-beta.2.tgz
```

## Documentación y soporte

- Contratos: [arquitectura](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/architecture.md),
  [CLI](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/cli.md) y
  [modelo de entrada](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/input-model.md).
- Errores y preguntas: [issues de GitHub](https://github.com/Kembec/facturacion-electronica-sunat-sdk/issues).

## Derechos y licencia

- Código de este repositorio: licencia MIT, © 2026 Manuel Benancio (ver [`LICENSE`](LICENSE)).
- La documentación técnica, los catálogos y los esquemas de la SUNAT le pertenecen a la SUNAT. Este repositorio no los
  redistribuye.
- Proyecto independiente: no está afiliado a la SUNAT, y no es un PSE ni un OSE. Quien lo usa sigue siendo el emisor
  electrónico y responde por sus comprobantes.
- Se entrega sin garantía, según la licencia MIT.
