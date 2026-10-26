# SDK de facturación electrónica SUNAT (Perú)

Un SDK para emitir comprobantes electrónicos ante la SUNAT desde **Rust, JavaScript y TypeScript, Python, PHP y Swift**,
o desde la línea de comandos. Tu sistema decide qué emitir y calcula los importes; el SDK convierte esos datos en el
archivo XML que pide la SUNAT, lo firma con tu certificado digital, lo envía y te dice si la SUNAT lo aceptó. Sirve
para facturas, boletas, notas de crédito y de débito, liquidaciones de compra, resúmenes diarios de boletas y
comunicaciones de baja para anular comprobantes ya enviados. Es para quien programa el sistema de facturación de una empresa.

SUNAT (Peru) electronic invoicing SDK for Rust, JavaScript/TypeScript, Python, PHP and Swift.

Todos los lenguajes usan el mismo motor por dentro, así que los mismos datos dan el mismo comprobante firmado en
cualquiera de ellos. Guías y ejemplos: **https://fe.kembec.com**

## Elige tu lenguaje

| Lenguaje | Paquete | Instalación | Cómo corre |
|---|---|---|---|
| JavaScript y TypeScript (Node 18 o posterior, navegador) | [`@kembec/facturacion-electronica-sunat-sdk`](https://www.npmjs.com/package/@kembec/facturacion-electronica-sunat-sdk) | `npm install @kembec/facturacion-electronica-sunat-sdk@beta` | WebAssembly, sin binarios ni procesos aparte |
| PHP 7.4 o posterior | [`kembec/facturacion-electronica-sunat-sdk`](https://packagist.org/packages/kembec/facturacion-electronica-sunat-sdk) | `composer require kembec/facturacion-electronica-sunat-sdk:^0.1@beta` | ejecuta el binario `sunat-sdk` una vez por llamada |
| Python 3.10 o posterior | [`facturacion-electronica-sunat-sdk`](https://pypi.org/project/facturacion-electronica-sunat-sdk/) | `pip install --pre facturacion-electronica-sunat-sdk` | wheels con el motor compilado adentro |
| Swift 5.9 o posterior (macOS, iOS, Linux) | el producto `KembecSunatSDK` de este repositorio | SwiftPM por URL, `exact: "1.0.0-rc.9"` ([cómo](bindings/swift/README.md#instalación)) | xcframework de bibliotecas estáticas, enlazado en tu binario; en Linux, `libsunat_sdk_ffi.so` |
| Rust | los paquetes de [`crates/`](crates) | desde este repositorio, [abajo](#rust) | crates, sin red |
| Línea de comandos | el programa `sunat-sdk` | [descarga en GitHub](https://github.com/Kembec/facturacion-electronica-sunat-sdk/releases) (Linux x86_64), o compílalo con `cargo build` | binario estático |

La versión publicada es la 1.0.0-rc.9. Qué trae, en [Versiones](#versiones).

## Tu primer comprobante firmado

Baja a una carpeta vacía una factura de ejemplo y el certificado de prueba del repositorio. Ese certificado es
autogenerado y no sirve para emitir.

```bash
base=https://raw.githubusercontent.com/Kembec/facturacion-electronica-sunat-sdk/v1.0.0-rc.9
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

Desde la 0.1.0-beta.5 los crates se publican en crates.io como `kembec-sunat-*`; también se usan como dependencia git
del tag, con la clave corta para que el código diga `sunat_model::…`. `sunat-model` lee y valida el documento,
`sunat-ubl` arma el XML y `sunat-xmldsig` lo firma.

```toml
[dependencies]
sunat-model = { package = "kembec-sunat-model", git = "https://github.com/Kembec/facturacion-electronica-sunat-sdk", tag = "v1.0.0-rc.9" }
sunat-ubl = { package = "kembec-sunat-ubl", git = "https://github.com/Kembec/facturacion-electronica-sunat-sdk", tag = "v1.0.0-rc.9" }
sunat-xmldsig = { package = "kembec-sunat-xmldsig", git = "https://github.com/Kembec/facturacion-electronica-sunat-sdk", tag = "v1.0.0-rc.9" }
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
url=https://github.com/Kembec/facturacion-electronica-sunat-sdk/releases/download/v1.0.0-rc.9
curl -fsSLO "$url/sunat-sdk-x86_64-unknown-linux-musl"
curl -fsSLO "$url/sunat-sdk-x86_64-unknown-linux-musl.sha256"
sha256sum -c sunat-sdk-x86_64-unknown-linux-musl.sha256
install -m 755 sunat-sdk-x86_64-unknown-linux-musl /usr/local/bin/sunat-sdk
sunat-sdk --version
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
| Liquidación de compra (04) | UBL 2.1 | arma, firma y envía con `send` ([abajo](#alcance)) |
| Resumen de reversión (RR) de liquidaciones de compra | UBL 2.0 | arma, firma, envía con `summary` al servicio `otroscpe` y consulta su ticket ([abajo](#alcance)) |
| Guía de remisión remitente (09) | UBL 2.1 | arma, firma y envía por la API REST de la SUNAT ([abajo](#alcance)) |
| Comprobante de retención (20) | UBL 2.0 | arma, firma y valida con los códigos de la SUNAT; la CLI lo envía a la beta ([abajo](#alcance)) |

La CDR es la constancia con la que la SUNAT responde a cada envío.

- **IGV** con las tasas de 18 % y 10,5 %: operaciones gravadas, exoneradas, inafectas, de exportación y gratuitas
  (afectaciones 10 a 16, 20, 21, 30 a 37 y 40), en soles y en dólares, al contado o a crédito en cuotas.
- **Resumen diario** con la tasa de IGV y el nombre del cliente en cada línea, como lo pide la SUNAT desde el
  01/08/2026, y con IVAP, ISC, otros tributos y la percepción del IGV en las líneas de boletas.
- **Descuentos y cargos** del catálogo 53, por línea y globales, en facturas y boletas.
- **ICBPER** (bolsas plásticas, tributo 7152), en comprobantes, notas y líneas del resumen diario.
- **Anticipos** en facturas y boletas: el comprobante del anticipo y el final que lo descuenta.
- **Exportación** de bienes y servicios (tipos de operación 0200 a 0208), en facturas y boletas.
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

- en comprobantes: ISC y otros tributos, descuentos y cargos en notas de crédito y débito, los códigos del catálogo 53
  que no son de descuentos y cargos, anticipos en notas y percepción;
- en la liquidación de compra: el tipo de operación 0502 (anticipos) y `totals.prepaid`, las series que empiezan con un
  dígito (las impresas), las líneas gratuitas (9996) y otros tributos (9999).

**Liquidación de compra (04)**: la emite el comprador, con su RUC, por una compra a una persona natural sin RUC; tipos
de operación 0501 y 0503 (compra de oro). Va con `kind: "self_billed"`, el vendedor (`seller`: DNI, carné de
extranjería o pasaporte), el lugar de la operación (`operation_place`) y la retención del impuesto a la renta en cada
línea. El SDK aplica las reglas de la SUNAT que solo leen el documento; las que consultan sus padrones (la condición
del vendedor, el tope de 75 UIT) las valida la SUNAT. Para darla de baja, el resumen de reversión (RR) es
`kind: "reversal"`: el nombre es `RUC-RR-YYYYMMDD-N` y la CLI lo envía con `summary` y lo consulta con `ticket` en el
servicio `otroscpe`, no en el de facturas, que lo rechaza con 0151. La SUNAT da 5 días desde la CDR de la liquidación;
el SDK solo ve la fecha de generación, así que rechaza con 2958 pasados 6 días, y un resumen enviado el día 6 aún puede
ser rechazado por ella. Los campos, en el [modelo de entrada](docs/input-model.md#purchase-settlement-04).

**Referencias**: la orden de compra (`order_reference`) y las guías de remisión que el comprobante cita
(`despatch_references`, tipo 09 o 31), en facturas, boletas y notas. Antes de enviar, el SDK rechaza lo que la SUNAT
observa (4005, 4006, 4233) y la guía citada dos veces (2364).

**Detracción**: en facturas y boletas con operación 1001 a 1004 (`detraction`: código de bien o servicio, porcentaje,
monto en soles, cuenta del Banco de la Nación y medio de pago). La 1002 (recursos hidrobiológicos) lleva en cada línea
la descarga (`fishing`: matrícula y nombre de la embarcación, especie, lugar, fecha y toneladas) y la 1004 (transporte
de carga), el viaje (`freight`: origen, destino, detalle y los tres valores referenciales). El monto lo calcula tu sistema; el SDK exige la
detracción con esas operaciones y la rechaza con cualquier otra.

**IVAP**: venta de arroz pilado en facturas, boletas y notas, con todas las líneas en afectación 17 y los totales
`ivap_base` e `ivap`. Las notas de crédito y débito van con el tipo 12, «Ajustes afectos al IVAP».

**Retención del IGV informada en el comprobante**: en facturas, boletas y notas (`igv_retention`: factor, importe y
base, cargo 62 del catálogo 53). El cliente debe tener RUC; que sea agente de retención lo valida la SUNAT contra su
padrón solo en producción.

**Guía de remisión remitente (09)**: se arma, se firma y se envía por la API REST de la SUNAT: en la CLI
(`gre-send`, que pide el token con la clave SOL y sube el zip con su `hashZip`, y `gre-ticket`, que consulta el ticket
hasta tener la CDR), en PHP (`greSend`, `greTicket`), en Python (`gre_send`, `gre_ticket`) y en npm, que arma cada
llamada y lee su respuesta para que tu `fetch` la haga (`greTokenRequest`, `greSendRequest`, `greTicketRequest` y sus
`*Response`). **La SUNAT no tiene ambiente de pruebas para la GRE**, y ninguna guía de este SDK figura como aceptada
por la SUNAT. Quedan fuera los traslados de importación y exportación y la guía del transportista (31).

El comprobante de percepción no forma parte del SDK. El detalle de cada campo
está en el [modelo de entrada](docs/input-model.md).

## Versiones

La 1.0.0-rc.9 agrega, sin cambiar ninguna salida:

- en npm, el módulo WebAssembly también se exporta como `./sdk.wasm`, para que un Cloudflare Worker lo cargue con la
  regla por defecto de wrangler; `./wasm` sigue apuntando al mismo archivo.

La 1.0.0-rc.8 agrega, sin cambiar ninguna salida que ya existía:

- en facturas (01) y boletas (03), la detracción de recursos hidrobiológicos (1002) y la de transporte de carga
  (1004), con los datos de línea que pide la SUNAT para cada una (`lines[].fishing` y `lines[].freight`). La SUNAT beta
  aceptó las tres de prueba con código 0 y sin observaciones;
- en el resumen diario (RC), líneas con IVAP en lugar de IGV, ISC, otros tributos y la percepción del IGV. La SUNAT
  beta aceptó los cuatro de prueba con código 0; el de IVAP, con la observación 4019, la misma línea que escribe
  Greenter;
- en npm, `npm install` sin versión instala esta candidata mientras no haya una versión estable.

La 1.0.0-rc.7 cambia frente a la 1.0.0-rc.6 solo en los registros: la página del paquete en npm, Packagist, NuGet y Maven
enlaza a [fe.kembec.com](https://fe.kembec.com). El SDK, su JSON de entrada y sus salidas no cambian.

La 1.0.0-rc.6 cambia frente a la 1.0.0-rc.5, sin campos nuevos en el JSON de entrada:

- en facturas (01) y boletas (03), el SDK revisa antes de firmar tres reglas de línea de la hoja de la SUNAT del
  24/4/2026, con tolerancia de 1: en una boleta, `line_value` = `unit_value` × `quantity` − descuentos `00` + cargos
  `47` (observación 4288); en toda línea, `unit_price` = (`line_value` + `total_taxes` − descuentos `01` + cargos `48`)
  ÷ `quantity`, salvo las gratuitas (3270 en factura, 4287 en boleta); y en toda línea con afectación 10 a 17,
  `igv` = `igv_base` × `igv_rate` (3103 en ambas). Antes se firmaban y la SUNAT las rechazaba o las aceptaba con
  observación; ahora se rechazan aquí, y los montos se corrigen en el origen. Cambio que rompe compatibilidad
  ([CHANGELOG](CHANGELOG.md));
- cada factura y boleta que el SDK arma desde `fixtures/` se valida, firmada, contra el esquema UBL 2.1 `Invoice` de
  OASIS, como ya se hacía con las guías de remisión.

La 1.0.0-rc.5 cambia frente a la 1.0.0-rc.4, sin campos nuevos en el JSON de entrada:

- en la guía de remisión remitente (09), el SDK rechaza antes de firmar el texto libre que la SUNAT observa por formato
  (la nota, las razones sociales, la descripción del motivo 13, el transportista y su registro MTC, los conductores, las
  direcciones de partida y llegada, y la descripción y el código de cada línea) cuando trae un salto de línea, un
  tabulador u otro espacio que no sea el normal, o se sale del largo de la SUNAT. La SUNAT no tiene ambiente de prueba
  para guías, así que la fuente es su hoja de validaciones. El valor se rechaza con su campo, nunca se reescribe. Cambio
  que rompe compatibilidad ([CHANGELOG](CHANGELOG.md));
- las notas de crédito (07) y débito (08) se revisan antes de firmar con `total_taxes` = `igv` + `ivap` + `icbper`, con
  tolerancia de 1, como las facturas y boletas (3294 y 4301). Una nota con el total descuadrado ya no se firma. Cambio
  que rompe compatibilidad ([CHANGELOG](CHANGELOG.md));
- `sunat-sdk --version` (o `-V`) imprime la versión en una línea y sale con 0.

La 1.0.0-rc.4 cambia frente a la 1.0.0-rc.3, sin campos nuevos en el JSON de entrada:

- en facturas, boletas y notas, el SDK rechaza antes de firmar la razón social, el nombre comercial, las partes de la
  dirección del emisor y el código de producto de una línea que traen un salto de línea, un tabulador u otro espacio
  que no sea el normal, o que se salen del largo que fija la SUNAT (4092, 4094 a 4098, 4338, 4234 y 4269). Antes
  llegaban a la SUNAT y volvían aceptados con una observación. El valor se rechaza con su campo, nunca se reescribe: se
  corrige en el origen. Cambio que rompe compatibilidad ([CHANGELOG](CHANGELOG.md));
- en el resumen diario, el IGV de las ventas gratuitas (9996) lleva la tasa de la línea; esas ventas entran al resumen
  desde el 1/1/2027, así que ningún resumen de hoy cambia.

La 1.0.0-rc.3 suma frente a la 1.0.0-rc.2; el JSON de entrada solo gana los tipos `self_billed` y `reversal`:

- la liquidación de compra (04), con los tipos de operación 0501 y 0503; la beta de la SUNAT aceptó con código 0 los dos
  vectores válidos;
- su baja por resumen de reversión (RR), que va por el servicio `otroscpe`: en la beta, el de facturas contestó 0151 y
  `otroscpe` dio un ticket con estado 0;
- el cierre de la exportación: los tipos de operación 0203 a 0207 con vectores propios, que la beta aceptó con código 0
  y sin notas; el catálogo 04 es la lista ISO 3166-1 alpha-2 vigente, así que un código como `ZZ`, `AN` o `CS` ahora se
  rechaza (3099 y 4280); y `igv_base` igual a `line_value` en todos los tipos de operación (3272 en facturas, 4294 en
  boletas);
- en toda factura y boleta, `totals.total_taxes` igual a `igv` + `ivap` + `icbper` antes de firmar (3294 en facturas, 4301
  en boletas); antes solo se revisaba en exportaciones y con ICBPER, y una factura común con el total descuadrado se
  firmaba y la SUNAT la rechazaba;
- el release adjunta `SunatSDKFFI.xcframework.zip`, el archivo cuya URL y huella nombra el `Package.swift` del tag, para
  Swift Package Manager;
- en el modelo de Rust, `Document` gana las variantes `SelfBilled` y `Reversal(Voided)`: un `match` sobre `Document`
  necesita sus brazos (o `_`). Cambio que rompe compatibilidad ([CHANGELOG](CHANGELOG.md)).

La 1.0.0-rc.2 suma frente a la 1.0.0-rc.1, sin cambiar el contrato JSON:

- la factura y la boleta de exportación en los nueve tipos de operación (0200 a 0208), con el país de uso del servicio
  y los datos del huésped en el hospedaje; la beta de la SUNAT aceptó los cuatro vectores válidos. En el modelo de
  Rust, `Line` e `Invoice` ganan campos;
- la respuesta de un OSE se lee como la de la SUNAT: número, código, observaciones y `diagnosis`.

La 1.0.0-rc.1 es la candidata a la primera versión estable: el mismo contrato que la 0.1.0-beta.8, con el paquete .NET
ya en NuGet (`Kembec.SunatSdk`). La 1.0.0 sale tras 15 días de uso en producción por un emisor real, con sus CDR
vigiladas y todo rechazo causado por el SDK corregido ([política de versiones](docs/releasing.md#versions)).

La 0.1.0-beta.8 había sumado frente a la 0.1.0-beta.7:

- anticipos en facturas y boletas: la del anticipo y la final que lo descuenta (`advances`, `totals.prepaid`), con los
  códigos de la SUNAT; la beta de la SUNAT aceptó los cuatro vectores válidos. En el modelo de Rust, `Invoice` y
  `Totals` ganan campos;
- en el resumen diario, `igv_rate` obligatorio en cada línea: sin ella el SDK responde 2992 antes de enviar. Cambio que
  rompe compatibilidad;
- cuando la SUNAT rechaza un resumen, la CLI devuelve su código y `diagnosis` en vez de `transport error: Bad Status`;
- el ICBPER, probado en la beta de la SUNAT (cinco vectores aceptados).

La 0.1.0-beta.7 había sumado frente a la 0.1.0-beta.6:

- el ICBPER (tributo 7152) en facturas, boletas, notas y resúmenes diarios;
- reglas con fecha de entrada en vigor del 01/01/2027 en el resumen diario y en la nota de débito 13;
- en la guía de remisión remitente (09) de transporte público, la fecha de entrega al transportista
  (`carrier_handover_date`), obligatoria: sin ella el SDK responde 3617. Cambio que rompe compatibilidad;
- `diagnosis` —el campo de entrada al que apunta cada código de la SUNAT— en todos los lenguajes: PHP, JavaScript,
  Python, .NET, Java, Ruby, Perl, Swift y Objective-C, y en la CLI (el CHANGELOG detalla cada uno).

La 0.1.0-beta.6 había sumado frente a la 0.1.0-beta.5:

- el comprobante de retención (20): armado, firma y validación con los códigos de la SUNAT, y envío por la CLI a la beta
  de la SUNAT, donde se aceptaron los tres vectores válidos;
- descuentos y cargos del catálogo 53 en facturas y boletas, por línea y globales;
- en el modelo de Rust, `Line`, `Invoice` y `Totals` ganan campos; el JSON de entrada no cambia sin descuentos ni cargos
  ([CHANGELOG](CHANGELOG.md)).

La 0.1.0-beta.5, publicada en npm, PyPI, Packagist y GitHub, había sumado frente a la 0.1.0-beta.4:

- el envío de la guía de remisión remitente (09) por la API REST de la SUNAT, en la CLI, PHP, Python y JavaScript;
- en PHP, los resúmenes diarios y las comunicaciones de baja (`summary`, `ticket`);
- la ABI en C y, sobre ella, .NET, Java, Ruby, Perl, Swift, Objective-C, VB6, Visual FoxPro y PowerBuilder; ninguno
  está todavía en su registro (NuGet, Maven Central, RubyGems, CPAN) ni se agrega por URL con SwiftPM;
- dos cambios que rompen compatibilidad: los crates se llaman `kembec-sunat-*`, y una CDR con código 0 y cualquier nota
  es `observed`. Cómo migrar, en el [CHANGELOG](CHANGELOG.md).

La 0.1.0-beta.4 había sumado frente a la 0.1.0-beta.3:

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
ejemplo de su manual, y el envío desde el navegador, que depende de que la SUNAT permita CORS.

## Documentación y soporte

- Guías y ejemplos: https://fe.kembec.com
- Contratos: [arquitectura](docs/architecture.md), [CLI](docs/cli.md) y [modelo de entrada](docs/input-model.md).
- Cambios de cada versión y qué rompe el contrato: [`CHANGELOG.md`](CHANGELOG.md) y la [política de versiones](docs/releasing.md#versions).
- Errores y preguntas: [issues de GitHub](https://github.com/Kembec/facturacion-electronica-sunat-sdk/issues).

## Derechos y licencia

- Código de este repositorio: licencia MIT, © 2026 Manuel Benancio (ver [`LICENSE`](LICENSE)).
- La documentación técnica, los catálogos y los esquemas de la SUNAT le pertenecen a la SUNAT. Este repositorio no los
  redistribuye.
- Proyecto independiente: no está afiliado a la SUNAT, y no es un PSE ni un OSE. Quien lo usa sigue siendo el emisor
  electrónico y responde por sus comprobantes.
- Se entrega sin garantía, según la licencia MIT.
