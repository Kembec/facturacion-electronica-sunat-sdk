# SDK de facturación electrónica SUNAT (Perú)

Un SDK (una biblioteca que agregas a tu programa) para emitir comprobantes electrónicos ante la SUNAT desde **Rust,
JavaScript y TypeScript, Python y PHP**, o desde la línea de comandos. Tu sistema decide qué emitir y calcula los
importes; el SDK convierte esos datos en el archivo XML que pide la SUNAT, lo firma con tu certificado digital, lo
envía y te dice si la SUNAT lo aceptó. Sirve para facturas, boletas, notas de crédito y de débito, resúmenes diarios
de boletas y comunicaciones de baja (las que anulan comprobantes ya enviados). Es para quien programa el sistema de
facturación de una empresa.

SUNAT (Peru) electronic invoicing SDK for Rust, JavaScript/TypeScript, Python and PHP.

Todos los lenguajes usan el mismo motor por dentro, así que los mismos datos dan el mismo comprobante firmado en
cualquiera de ellos. Guías y ejemplos: **https://fe.kembec.com**

## Elige tu lenguaje

| Lenguaje | Paquete | Instalación | Cómo corre |
|---|---|---|---|
| JavaScript y TypeScript (Node 18 o posterior, navegador) | [`@kembec/facturacion-electronica-sunat-sdk`](https://www.npmjs.com/package/@kembec/facturacion-electronica-sunat-sdk) | `npm install @kembec/facturacion-electronica-sunat-sdk@beta` | WebAssembly, sin binarios ni procesos aparte |
| PHP 7.4 o posterior | [`kembec/facturacion-electronica-sunat-sdk`](https://packagist.org/packages/kembec/facturacion-electronica-sunat-sdk) | `composer require kembec/facturacion-electronica-sunat-sdk:^0.1@beta` | ejecuta el binario `sunat-sdk` una vez por llamada |
| Python 3.10 o posterior | [`facturacion-electronica-sunat-sdk`](https://pypi.org/project/facturacion-electronica-sunat-sdk/) | `pip install --pre facturacion-electronica-sunat-sdk` | wheels con el motor compilado adentro |
| Rust | los paquetes de [`crates/`](crates) | desde este repositorio, [abajo](#rust) | crates, sin red |
| Línea de comandos | el programa `sunat-sdk` | [descarga en GitHub](https://github.com/Kembec/facturacion-electronica-sunat-sdk/releases) (Linux x86_64), o compílalo con `cargo build` | binario estático |

La versión publicada es la 0.1.0-beta.4, y por ser beta las instalaciones de arriba la piden de forma explícita
(`@beta` en npm y Composer, `--pre` en pip). Qué trae, en [Versiones](#versiones).

## Tu primer comprobante firmado

Baja a una carpeta vacía una factura de ejemplo y el certificado de prueba del repositorio. Ese certificado es
autogenerado y no sirve para emitir.

```bash
base=https://raw.githubusercontent.com/Kembec/facturacion-electronica-sunat-sdk/v0.1.0-beta.4
curl -fsSLO "$base/fixtures/01-invoice-taxed-cash.json"
curl -fsSLO "$base/tests/vectors/test-cert/test-cert.pem"
curl -fsSLO "$base/tests/vectors/test-cert/test-key.pem"
```

Cada ejemplo de abajo arma esa factura, la firma e imprime el nombre del archivo y la huella de la firma
(`digest_value`). Si ves `20123456789-01-F001-1 +te61a2ub5MlTFf8tKTPNGWEXKI=`, el SDK funciona: es la misma salida en
todos los lenguajes. Tus propios documentos siguen la forma del [modelo de entrada](docs/input-model.md).

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

El paquete de PHP ejecuta el binario `sunat-sdk`. En Linux x86_64, `vendor/bin/sunat-sdk-install` baja el binario de
su misma versión, comprueba su SHA-256 e imprime `SUNAT_SDK_BIN=<ruta>`. En otra plataforma, compílalo como en
[Línea de comandos](#línea-de-comandos).

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

Los crates no están en crates.io: se usan como dependencia git de la versión publicada en GitHub. `sunat-model` lee y
valida el documento, `sunat-ubl` arma el XML y `sunat-xmldsig` lo firma.

```toml
[dependencies]
sunat-model = { git = "https://github.com/Kembec/facturacion-electronica-sunat-sdk", tag = "v0.1.0-beta.4" }
sunat-ubl = { git = "https://github.com/Kembec/facturacion-electronica-sunat-sdk", tag = "v0.1.0-beta.4" }
sunat-xmldsig = { git = "https://github.com/Kembec/facturacion-electronica-sunat-sdk", tag = "v0.1.0-beta.4" }
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

Los crates no hacen red: `sunat-transport` arma el ZIP y la petición SOAP y lee la respuesta, y el POST HTTP lo haces
tú. Lo que expone cada crate está en la [arquitectura](docs/architecture.md).

### Línea de comandos

En Linux x86_64, el binario estático del release:

```bash
url=https://github.com/Kembec/facturacion-electronica-sunat-sdk/releases/download/v0.1.0-beta.4
curl -fsSLO "$url/sunat-sdk-x86_64-unknown-linux-musl"
curl -fsSLO "$url/sunat-sdk-x86_64-unknown-linux-musl.sha256"
sha256sum -c sunat-sdk-x86_64-unknown-linux-musl.sha256
install -m 755 sunat-sdk-x86_64-unknown-linux-musl /usr/local/bin/sunat-sdk
```

En otra plataforma, compílalo con Rust 1.94.1:

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
| Guía de remisión remitente (09) | UBL 2.1 | arma y firma; el envío está solo en `main` ([abajo](#alcance)) |

La CDR es la constancia con la que la SUNAT responde a cada envío.

- **IGV** con las tasas de 18 % y 10,5 %: operaciones gravadas, exoneradas, inafectas, de exportación y gratuitas
  (afectaciones 10 a 16, 20, 21, 30 a 37 y 40), en soles y en dólares, al contado o a crédito en cuotas.
- **Resumen diario** con la tasa de IGV y el nombre del cliente en cada línea, como lo pide la SUNAT desde el
  01/08/2026.
- **Descuento por línea** que reduce la base del IGV (catálogo 53, código 00), en facturas y boletas.
- **Firma** XML-DSig enveloped como la pide la SUNAT (RSA-SHA1, C14N inclusiva), con tu certificado y tu llave en PEM;
  la llave, RSA sin cifrar.
- **Respuesta de la SUNAT**: el SDK lee la CDR y la clasifica (aceptada, observada, rechazada, excepción). Un error SOAP
  es un resultado, no una excepción.
- **Sin respuesta**: si una consulta no recibe respuesta, el resultado es `no_verdict`; vuelve a consultar. Con el
  ticket de un resumen o una baja, nunca reenvíes el archivo: el ticket prueba que llegó. Si una factura, boleta o nota
  enviada sola sigue sin veredicto, reenvía el mismo XML firmado con el mismo número mientras el plazo de envío siga
  abierto, porque la SUNAT lo mide al recibirlo; según sus reglas de validación, si ya lo tenía no lo registra dos
  veces y contesta 1033, 1032 o 0140. Un `summary` sin respuesta no deja ticket: revisa los comprobantes con `validity`
  y reenvía el mismo archivo con el mismo nombre ([contrato de la CLI](docs/cli.md)).
- **Estado de un comprobante ya enviado** en la Consulta Integrada de la SUNAT (`validity`: aceptado, anulado o no
  informado), para facturas, boletas y notas.

Cómo llega cada lenguaje a la SUNAT:

| | Arma, firma, ZIP y CDR | Envía y consulta |
|---|---|---|
| Línea de comandos | `build`, `sign`, `zip`, `cdr` | `send`, `summary`, `ticket`, `status`, `validity`, con HTTP y TLS verificado |
| PHP | `build`, `sign`, `zip`, `cdr` | `send`, `status` y `validity`; resúmenes y bajas, con la CLI |
| JavaScript y TypeScript | `build`, `sign`, `zip`, `cdr`, en memoria | `soapRequest` arma el POST, tu `fetch` lo envía y `soapResponse` lee la respuesta |
| Python | `build`, `sign`, `zip`, `cdr`, en memoria | `exchange` hace el POST con TLS verificado; `soap_request` y `soap_response` para tu propio cliente HTTP |
| Rust | `sunat-model`, `sunat-ubl`, `sunat-xmldsig`, `sunat-transport` | `sunat_sdk::exchange` arma la petición y lee la respuesta; el HTTP es tuyo |

## Alcance

**Los importes los calcula tu sistema.** Llegan como texto (`"118.00"`); el SDK los valida y los escribe, no calcula
impuestos. Con descuento por línea, `unit_value` es el valor antes del descuento y `line_value` es `unit_value` ×
`quantity` menos los descuentos. Si mandas un `unit_value` ya neto del descuento, la SUNAT rechaza la factura (3271) y
observa la boleta (4288).

**Lo que el SDK no cubre lo rechaza con un error explícito**, en vez de generar un XML que la SUNAT rechazaría:

- en comprobantes: ISC, ICBPER y otros tributos, descuentos y cargos globales, cargos por línea, descuentos por
  línea con un código distinto de 00 o en notas de crédito y débito, anticipos, detracción en operaciones 1002 y 1004 y percepción;
- en resúmenes diarios: percepción, ICBPER, ISC e IVAP en las líneas, y los resúmenes de reversión (RR).

**Referencias**: la orden de compra (`order_reference`) y las guías de remisión que el comprobante cita
(`despatch_references`, tipo 09 o 31), en facturas, boletas y notas. Antes de enviar, el SDK rechaza lo que la SUNAT
observa (4005, 4006, 4233) y la guía citada dos veces (2364).

**Detracción**: en facturas y boletas con operación 1001 o 1003 (`detraction`: código de bien o servicio, porcentaje,
monto en soles, cuenta del Banco de la Nación y medio de pago). El monto lo calcula tu sistema; el SDK exige la
detracción con esas operaciones y la rechaza con cualquier otra.

**IVAP**: venta de arroz pilado en facturas, boletas y notas, con todas las líneas en afectación 17 y los totales
`ivap_base` e `ivap`. Las notas de crédito y débito van con el tipo 12, «Ajustes afectos al IVAP».

**Retención del IGV informada en el comprobante**: en facturas, boletas y notas (`igv_retention`: factor, importe y
base, cargo 62 del catálogo 53). El cliente debe tener RUC; que sea agente de retención lo valida la SUNAT contra su
padrón solo en producción.

**Guía de remisión remitente (09)**: la 0.1.0-beta.4 la arma y la firma. Su envío por la API REST de la SUNAT está en
la rama `main`, todavía sin publicar: en la CLI (`gre-send`, que pide el token con la clave SOL y sube el zip con su
`hashZip`, y `gre-ticket`, que consulta el ticket hasta tener la CDR), en PHP (`greSend`, `greTicket`), en Python
(`gre_send`, `gre_ticket`) y en npm, que arma cada llamada y lee su respuesta para que tu `fetch` la haga
(`greTokenRequest`, `greSendRequest`, `greTicketRequest` y sus `*Response`). **La SUNAT no tiene ambiente de pruebas
para la GRE**, y ninguna guía de este SDK figura como aceptada por la SUNAT. Quedan fuera los traslados de importación
y exportación y la guía del transportista (31).

El comprobante de retención y el de percepción no forman parte del SDK. El detalle de cada campo
está en el [modelo de entrada](docs/input-model.md).

## Versiones

La 0.1.0-beta.4, publicada en npm, PyPI, Packagist y GitHub, suma frente a la 0.1.0-beta.3:

- la consulta de estado de comprobantes (`validity`, Consulta Integrada) para facturas, boletas y notas, en la CLI, PHP,
  JavaScript y Python;
- la detracción en facturas y boletas con operación 1001 y 1003;
- la retención del IGV informada en el comprobante (cargo 62 del catálogo 53);
- el IVAP, en facturas, boletas y notas;
- la orden de compra y las guías de remisión relacionadas;
- la guía de remisión remitente (09): armado, firma y validación, sin envío.

La 0.1.0-beta.3 había sumado los resúmenes diarios y las comunicaciones de baja, el descuento por línea, el envío y las
consultas desde JavaScript, la CDR observada (código 0 con una nota de 4000 o más), `no_verdict` y el paquete de Python.

## Cómo se comprueba

Un tipo de documento entra al SDK cuando cumple tres condiciones:

1. su XML coincide con los vectores de [`tests/vectors/`](tests/vectors);
2. su firma da exactamente el mismo `DigestValue` y el mismo `SignatureValue`, y la acepta un verificador independiente;
3. la beta de la SUNAT lo acepta.

**Qué aceptó la SUNAT.** El 16/10/2026, la beta de la SUNAT aceptó 36 de los 38 documentos de
[`fixtures/`](fixtures), enviados con la CLI: los 29 comprobantes, con código 0 y sin observaciones, y los 7 resúmenes
diarios y comunicaciones de baja, procesados con código 0. Las 2 guías de remisión no pasan por la beta, porque la
SUNAT no tiene ambiente de pruebas para la GRE: se contrastan con sus vectores, el esquema OASIS y las reglas de la
SUNAT, y su envío se probó solo contra respuestas del manual de la SUNAT y de un cliente de terceros.

**En cada cambio**, el CI arma y firma los 38 documentos por cada vía —la CLI, PHP, el paquete npm en Node 18, Node 22 y
Chrome, y el paquete de Python instalado— y exige el `DigestValue` y el `SignatureValue` exactos de su vector. Un
verificador en Python que no comparte código con el núcleo acepta las firmas.

**Sin red**: el envío y las consultas desde JavaScript y Python se prueban con respuestas escritas a mano; comparten con
la CLI el código que arma la petición y lee la respuesta.

**Sin probar contra la SUNAT**: la consulta de la CDR (`status`), que la SUNAT solo ofrece en producción y que sigue el
ejemplo de su manual, y el envío desde el navegador, que depende de que la SUNAT acepte peticiones de otro origen
(CORS).

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
