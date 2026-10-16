# SDK de facturación electrónica SUNAT (Perú)

Un SDK (una biblioteca que agregas a tu programa) para emitir comprobantes electrónicos ante la SUNAT desde **Rust,
JavaScript y TypeScript, Python y PHP**, o desde la línea de comandos. Tu sistema decide qué emitir y calcula los
importes; el SDK convierte esos datos en el archivo XML que pide la SUNAT, lo firma con tu certificado digital, lo
envía y te dice si la SUNAT lo aceptó. Sirve para facturas, boletas, notas de crédito y de débito, resúmenes diarios
de boletas y comunicaciones de baja (las que anulan comprobantes ya enviados). Es para quien programa el sistema de
facturación de una empresa.

SUNAT (Peru) electronic invoicing SDK for Rust, JavaScript/TypeScript, Python and PHP.

Todos los lenguajes usan el mismo motor por dentro, así que los mismos datos dan el mismo comprobante firmado en
cualquiera de ellos. Las pruebas automáticas lo comprueban en cada cambio con 22 documentos de ejemplo, y el entorno de
pruebas de la SUNAT aceptó esos documentos el 16/10/2026.

Guías y ejemplos: **https://fe.kembec.com**

## Elige tu lenguaje

| Lenguaje | Paquete | Instalación |
|---|---|---|
| JavaScript y TypeScript (Node 18 o posterior, navegador) | [`@kembec/facturacion-electronica-sunat-sdk`](https://www.npmjs.com/package/@kembec/facturacion-electronica-sunat-sdk) | `npm install @kembec/facturacion-electronica-sunat-sdk@beta` |
| PHP 7.4 o posterior | [`kembec/facturacion-electronica-sunat-sdk`](https://packagist.org/packages/kembec/facturacion-electronica-sunat-sdk) | `composer require kembec/facturacion-electronica-sunat-sdk:^0.1@beta` |
| Python 3.10 o posterior | [`facturacion-electronica-sunat-sdk`](https://pypi.org/project/facturacion-electronica-sunat-sdk/) | `pip install --pre facturacion-electronica-sunat-sdk` |
| Rust | los paquetes de [`crates/`](crates) | desde este repositorio, [abajo](#rust) |
| Línea de comandos | el programa `sunat-sdk` | [descarga en GitHub](https://github.com/Kembec/facturacion-electronica-sunat-sdk/releases) (Linux x86_64), o compílalo con `cargo build` |

Este README describe la 0.1.0-beta.3, publicada en npm, PyPI, Packagist y GitHub. Es una versión beta: por eso las
instalaciones de arriba la piden de forma explícita (`@beta` en npm y Composer, `--pre` en pip). Lo que cambió desde la
anterior está en [Versiones](#versiones).

## Tu primer comprobante, en cualquier lenguaje

La misma factura de prueba, firmada con el certificado de prueba del repositorio, da en todos los lenguajes la misma
huella de la firma (su `DigestValue`), `+te61a2ub5MlTFf8tKTPNGWEXKI=`: la del archivo de referencia
[`tests/vectors/01-invoice-taxed-cash.signed.xml`](tests/vectors/01-invoice-taxed-cash.signed.xml). Baja los tres
archivos a una carpeta vacía:

```bash
base=https://raw.githubusercontent.com/Kembec/facturacion-electronica-sunat-sdk/v0.1.0-beta.3
curl -fsSLO "$base/fixtures/01-invoice-taxed-cash.json"
curl -fsSLO "$base/tests/vectors/test-cert/test-cert.pem"
curl -fsSLO "$base/tests/vectors/test-cert/test-key.pem"
```

Ese certificado es autogenerado y no sirve para emitir. El documento de entrada es un JSON con la forma del
[modelo de entrada](docs/input-model.md).

### JavaScript y TypeScript

```js
// primer-comprobante.mjs, después de npm install @kembec/facturacion-electronica-sunat-sdk@beta
import { readFileSync } from 'node:fs';
import { load } from '@kembec/facturacion-electronica-sunat-sdk';

const read = (file) => readFileSync(file, 'utf8');
const sdk = await load();
const { file_name, xml } = JSON.parse(sdk.build(read('01-invoice-taxed-cash.json')));
const signed = JSON.parse(sdk.sign(xml, read('test-cert.pem'), read('test-key.pem')));
console.log(file_name, signed.digest_value); // 20123456789-01-F001-1 +te61a2ub5MlTFf8tKTPNGWEXKI=
```

Los tipos vienen en el paquete. El envío a la SUNAT, el uso en el navegador y en Cloudflare Workers están en el
[README del paquete](bindings/wasm/npm/README.md).

### PHP

El paquete de PHP ejecuta el binario `sunat-sdk`. En Linux x86_64, `vendor/bin/sunat-sdk-install` lo descarga del
release de su misma versión, comprueba su SHA-256 e imprime `SUNAT_SDK_BIN=<ruta>`; en otra plataforma, compílalo como
en [Línea de comandos](#línea-de-comandos).

```bash
composer require kembec/facturacion-electronica-sunat-sdk:^0.1@beta
export "$(vendor/bin/sunat-sdk-install)"
```

```php
<?php // primer-comprobante.php
require 'vendor/autoload.php';

use Kembec\SunatSdk\SigningKey;
use Kembec\SunatSdk\SunatSdk;

$sdk = new SunatSdk(getenv('SUNAT_SDK_BIN'), 120.0); // ruta del binario y tiempo máximo por llamada, en segundos
$built = $sdk->build(file_get_contents('01-invoice-taxed-cash.json'));
$signed = $sdk->sign($built->getXml(), new SigningKey('test-cert.pem', 'test-key.pem'));
echo $built->getFileName(), ' ', $signed->getDigestValue(), PHP_EOL; // 20123456789-01-F001-1 +te61a2ub5MlTFf8tKTPNGWEXKI=
```

El envío, la consulta de la CDR y los errores están en el [README del cliente PHP](bindings/php/README.md).

### Python

```python
# primer_comprobante.py
import sunat_sdk

def read(file):
    with open(file, encoding="utf-8") as f:
        return f.read()

built = sunat_sdk.build(read("01-invoice-taxed-cash.json"))
signed = sunat_sdk.sign(built["xml"], read("test-cert.pem"), read("test-key.pem"))
print(built["file_name"], signed["digest_value"])  # 20123456789-01-F001-1 +te61a2ub5MlTFf8tKTPNGWEXKI=
```

La instalación y el envío a la SUNAT (`exchange`) están en el [README del paquete de Python](bindings/python/README.md).

### Rust

Los crates no están en crates.io: se usan como dependencia git del tag. `sunat-model` lee y valida el documento,
`sunat-ubl` arma el XML y `sunat-xmldsig` lo firma.

```toml
[dependencies]
sunat-model = { git = "https://github.com/Kembec/facturacion-electronica-sunat-sdk", tag = "v0.1.0-beta.3" }
sunat-ubl = { git = "https://github.com/Kembec/facturacion-electronica-sunat-sdk", tag = "v0.1.0-beta.3" }
sunat-xmldsig = { git = "https://github.com/Kembec/facturacion-electronica-sunat-sdk", tag = "v0.1.0-beta.3" }
```

```rust
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let document = sunat_model::Document::from_json(&fs::read_to_string("01-invoice-taxed-cash.json")?)?;
    let unsigned = sunat_ubl::build(&document)?;
    let signed = sunat_xmldsig::sign(
        &unsigned,
        &fs::read_to_string("test-cert.pem")?,
        &fs::read_to_string("test-key.pem")?,
    )?;
    fs::write(format!("{}.xml", document.file_name()), signed)?; // 20123456789-01-F001-1.xml
    Ok(())
}
```

El XML firmado lleva el mismo `<ds:DigestValue>` del vector. El núcleo no hace red: `sunat-transport` arma el ZIP y
los sobres SOAP y lee las respuestas, y `sunat-sdk` da las llamadas al web service en el JSON de la CLI; el POST HTTP lo
haces tú. Lo que expone cada crate está en la [arquitectura](docs/architecture.md).

### Línea de comandos

En Linux x86_64, el binario estático del release:

```bash
url=https://github.com/Kembec/facturacion-electronica-sunat-sdk/releases/download/v0.1.0-beta.3
curl -fsSLO "$url/sunat-sdk-x86_64-unknown-linux-musl"
curl -fsSLO "$url/sunat-sdk-x86_64-unknown-linux-musl.sha256"
sha256sum -c sunat-sdk-x86_64-unknown-linux-musl.sha256
install -m 755 sunat-sdk-x86_64-unknown-linux-musl /usr/local/bin/sunat-sdk
```

En otra plataforma, compílalo con Rust (el CI usa la 1.94.1):

```bash
git clone https://github.com/Kembec/facturacion-electronica-sunat-sdk.git
cargo build --manifest-path facturacion-electronica-sunat-sdk/Cargo.toml -p sunat-sdk-cli --release --locked
# el binario queda en facturacion-electronica-sunat-sdk/target/release/sunat-sdk
```

```bash
sunat-sdk build < 01-invoice-taxed-cash.json \
  | sunat-sdk sign --cert test-cert.pem --key test-key.pem \
  | grep -o '"digest_value":"[^"]*"'
# "digest_value":"+te61a2ub5MlTFf8tKTPNGWEXKI="
```

Cada comando lee un JSON por la entrada estándar y escribe un JSON por la salida. `send`, `summary`, `ticket` y
`status` toman la URL del web service y las credenciales SOL de opciones o del entorno, nunca del código. El contrato
completo está en [`docs/cli.md`](docs/cli.md).

## Qué cubre

| Documento | Formato | Qué hace el SDK |
|---|---|---|
| Factura (01), boleta (03), nota de crédito (07) y nota de débito (08) | UBL 2.1 | arma, firma, comprime, envía (`sendBill`) y consulta la CDR (`getStatusCdr`) |
| Resumen diario (RC) de boletas y sus notas | UBL 2.0 | arma, firma, envía (`sendSummary`) y consulta el ticket (`getStatus`) |
| Comunicación de baja (RA) de facturas y sus notas | UBL 2.0 | arma, firma, envía (`sendSummary`) y consulta el ticket (`getStatus`) |

- **IGV** con las tasas de 18 % y 10,5 %: operaciones gravadas, exoneradas, inafectas, de exportación y gratuitas
  (afectaciones 10 a 16, 20, 21, 30 a 37 y 40), en soles y en dólares, al contado o a crédito en cuotas.
- **Descuento por línea** que reduce la base del IGV (catálogo 53, código 00), en facturas y boletas.
- **Firma** XML-DSig enveloped como la pide la SUNAT (RSA-SHA1, C14N inclusiva), con tu certificado y tu llave en PEM;
  la llave, RSA sin cifrar.
- **Respuesta de la SUNAT**: el SDK lee la CDR y la clasifica (aceptada, observada, rechazada, excepción). Un error SOAP
  es un resultado, no una excepción, y una consulta que no recibe respuesta dice `no_verdict`: se vuelve a consultar y
  nunca se reenvía.

Todos los lenguajes reciben el mismo documento de entrada; lo que cambia es cómo llega a la SUNAT:

| | Arma, firma, ZIP y CDR | Envía y consulta |
|---|---|---|
| Línea de comandos | `build`, `sign`, `zip`, `cdr` | `send`, `summary`, `ticket`, `status`, con HTTP y TLS verificado |
| PHP | `build`, `sign`, `zip`, `cdr` | `send` y `status`; resúmenes y bajas, con la CLI |
| JavaScript y TypeScript | `build`, `sign`, `zip`, `cdr`, en memoria | `soapRequest` arma el POST, tu `fetch` lo envía y `soapResponse` lee la respuesta |
| Python | `build`, `sign`, `zip`, `cdr`, en memoria | `exchange` hace el POST con TLS verificado; `soap_request` y `soap_response` para tu propio cliente HTTP |
| Rust | `sunat-model`, `sunat-ubl`, `sunat-xmldsig`, `sunat-transport` | `sunat_sdk::exchange` arma la petición y lee la respuesta; el HTTP es tuyo |

## Cómo funciona por dentro

El SDK arma XML UBL (2.1 en comprobantes, 2.0 en resúmenes y bajas), lo firma con XML-DSig, habla con el web service
SOAP de la SUNAT y lee la CDR, la constancia con la que la SUNAT responde. Todo eso lo hace un mismo núcleo en Rust, que
cada lenguaje recibe así:

- **JavaScript y TypeScript**: el núcleo compilado a WebAssembly, sin binarios ni procesos aparte.
- **PHP**: un cliente del binario `sunat-sdk`, que lo ejecuta una vez por llamada.
- **Python**: wheels con el núcleo compilado adentro; no llaman a ningún binario externo.
- **Rust**: los crates de [`crates/`](crates), como dependencia git del tag (no están en crates.io).
- **Línea de comandos**: el binario `sunat-sdk`.

## Cómo se comprueba

Un tipo de documento entra al SDK cuando cumple tres condiciones: su XML coincide con los vectores de
[`tests/vectors/`](tests/vectors), su firma da exactamente el mismo `DigestValue` y el mismo `SignatureValue` y la acepta
un verificador independiente, y la beta de la SUNAT lo acepta.

- **En cada cambio**, el CI arma y firma los 34 casos de [`fixtures/`](fixtures) por la CLI, por PHP, por el paquete
  npm en Node 18, Node 22 y Chrome, y por el paquete de Python instalado, y exige el `DigestValue` y el `SignatureValue`
  exactos de su vector. Un verificador en Python que no comparte código con el núcleo acepta las firmas.
- **En la beta de la SUNAT**, el 16/10/2026, enviados con la CLI: los 15 comprobantes, aceptados con código 0 y sin
  observaciones; los 7 resúmenes y bajas, procesados con código 0.
- **Sin red**, con respuestas escritas a mano: el envío y las consultas desde JavaScript y Python, que comparten con la
  CLI el código que arma la petición y lee la respuesta. Los envíos a la beta se hicieron con la CLI.
- La consulta de la CDR (`status`) sigue el ejemplo del manual; la SUNAT solo la ofrece en producción, así que no se ha
  probado contra ella. Tampoco el envío desde el navegador, que depende de que la SUNAT acepte peticiones de otro origen
  (CORS).

## Alcance

**Los importes los calcula tu sistema.** Llegan como texto (`"118.00"`); el SDK los valida y los escribe, no calcula
impuestos. Con descuento por línea, `unit_value` es el valor antes del descuento y `line_value` es `unit_value` ×
`quantity` menos los descuentos: en la beta, un `unit_value` ya neto del descuento dio el rechazo 3271 en la factura y
la observación 4288 en la boleta.

**Lo que el SDK no cubre lo rechaza con un error explícito**, en vez de generar un XML que la SUNAT rechazaría:

- en comprobantes: ISC, ICBPER y otros tributos, descuentos y cargos globales, cargos por línea, descuentos por
  línea con un código distinto de 00 o en notas de crédito y débito, anticipos, detracción en operaciones 1002 y 1004 y percepción;
- en resúmenes diarios: percepción, ICBPER, ISC e IVAP en las líneas, y los resúmenes de reversión (RR).

**Referencias**: la orden de compra (`order_reference`) y las guías de remisión que el comprobante cita
(`despatch_references`, tipo 09 o 31), en facturas, boletas y notas. El SDK rechaza lo que la beta observó (4005, 4006,
4233) y la guía repetida (2364).

**Detracción**: en facturas y boletas con operación 1001 o 1003 (`detraction`: código de bien o servicio, porcentaje,
monto en soles, cuenta del Banco de la Nación y medio de pago). El monto lo calcula tu sistema; el SDK exige la
detracción con esas operaciones y la rechaza con cualquier otra.

**IVAP**: venta de arroz pilado en facturas, boletas y notas, con todas las líneas en afectación 17 y los totales
`ivap_base` e `ivap`. Las notas de crédito y débito van con el tipo 12, «Ajustes afectos al IVAP».

La guía de remisión, el comprobante de retención y el de percepción no forman parte del SDK. El detalle de cada campo
está en el [modelo de entrada](docs/input-model.md).

## Versiones

Un tag publica a la vez el binario, el paquete npm, el de PHP y el de Python ([`docs/releasing.md`](docs/releasing.md)).
La 0.1.0-beta.3 es la primera que llega a PyPI. Frente a la 0.1.0-beta.2, que armaba, firmaba, comprimía, enviaba y
consultaba facturas, boletas y notas desde la CLI y PHP, y solo las armaba y firmaba desde JavaScript, trae:

- resúmenes diarios y comunicaciones de baja (`build`, y `summary` y `ticket` en la CLI);
- en las líneas del resumen diario, la tasa de IGV y el nombre del cliente, que la SUNAT pide desde el 01/08/2026;
- descuento por línea;
- `zip`, `cdr` y el envío y las consultas desde JavaScript;
- `no_verdict` en `ticket` y `status`, y la CDR observada (código 0 con una nota de 4000 o más);
- el paquete de Python.

## Documentación y soporte

- Guías y ejemplos: https://fe.kembec.com
- Contratos: [arquitectura](docs/architecture.md), [CLI](docs/cli.md) y [modelo de entrada](docs/input-model.md).
- Errores y preguntas: [issues de GitHub](https://github.com/Kembec/facturacion-electronica-sunat-sdk/issues).

## Derechos y licencia

- Código de este repositorio: licencia MIT, © 2026 Manuel Benancio (ver [`LICENSE`](LICENSE)).
- La documentación técnica, los catálogos y los esquemas de la SUNAT le pertenecen a la SUNAT. Este repositorio no los
  redistribuye.
- Proyecto independiente: no está afiliado a la SUNAT, y no es un PSE ni un OSE. Quien lo usa sigue siendo el emisor
  electrónico y responde por sus comprobantes.
- Se entrega sin garantía, según la licencia MIT.
