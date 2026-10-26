# SDK de facturación electrónica SUNAT (Perú) para PHP

Un SDK para emitir comprobantes electrónicos ante la SUNAT desde **PHP**, para quien programa el sistema de facturación
de una empresa. Con los datos de una factura (01), una boleta (03) o una nota de crédito (07) o de débito (08):

- arma el archivo XML que pide la SUNAT y lo firma con tu certificado digital;
- lo comprime, lo envía y lee la CDR, la constancia con la que la SUNAT dice si lo aceptó;
- después consulta un comprobante ya enviado: su CDR (`status`) o su estado en la Consulta Integrada de la SUNAT
  (`validity`: aceptado, anulado o no informado).

También arma y firma resúmenes diarios de boletas (RC), comunicaciones de baja (RA) y guías de remisión remitente (09).
Los resúmenes y las bajas se envían con `summary` y su resultado se consulta con `ticket`; las guías, solo desde la rama
`main` ([abajo](#guía-de-remisión-remitente-09-por-la-api-rest)). Es la versión para PHP del
[SDK](https://github.com/Kembec/facturacion-electronica-sunat-sdk), que también funciona en JavaScript y TypeScript,
Python y Rust: los mismos datos dan el mismo comprobante firmado en todos.

SUNAT (Peru) electronic invoicing SDK for PHP.

Requiere PHP 7.4 o posterior con la extensión `json`; el instalador usa además `openssl`. Tu sistema calcula los
importes; el SDK los valida y los pone en el XML, pero no calcula impuestos. Guías y ejemplos: https://fe.kembec.com

## Instalación

```bash
composer require kembec/facturacion-electronica-sunat-sdk:^0.1@beta
export "$(vendor/bin/sunat-sdk-install)"     # opcional: --dir=<directorio>
```

Las versiones publicadas son beta.

El trabajo lo hace el binario `sunat-sdk`, el núcleo en Rust del SDK: este paquete es su cliente y lo ejecuta una vez
por llamada. El contrato de cada comando está en el
[contrato de la CLI](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/cli.md).

`sunat-sdk-install` baja de GitHub el binario de la misma versión del paquete, para Linux x86_64, y lo deja ejecutable
solo si su SHA-256 coincide. Imprime `SUNAT_SDK_BIN=<ruta absoluta>`: esa ruta es el primer argumento de `SunatSdk`.
Por defecto lo deja en `vendor/kembec/facturacion-electronica-sunat-sdk/bindings/php/bin/`; usa `--dir` si `vendor/`
es de solo lectura al ejecutar. Composer no lo corre por su cuenta. En otra plataforma falla e indica cómo compilar el
binario con Rust:

```bash
git clone https://github.com/Kembec/facturacion-electronica-sunat-sdk.git
cargo build --manifest-path facturacion-electronica-sunat-sdk/Cargo.toml -p sunat-sdk-cli --release --locked
export SUNAT_SDK_BIN="$PWD/facturacion-electronica-sunat-sdk/target/release/sunat-sdk"
```

## Tu primer comprobante

Baja a la carpeta de tu proyecto la factura de prueba y el certificado de prueba del repositorio. Ese certificado es
autogenerado y no sirve para emitir.

```bash
base=https://raw.githubusercontent.com/Kembec/facturacion-electronica-sunat-sdk/v1.0.0-rc.8
curl -fsSLO "$base/fixtures/01-invoice-taxed-cash.json"
curl -fsSLO "$base/tests/vectors/test-cert/test-cert.pem"
curl -fsSLO "$base/tests/vectors/test-cert/test-key.pem"
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

Si imprime `20123456789-01-F001-1 +te61a2ub5MlTFf8tKTPNGWEXKI=` (el nombre del archivo y la huella de la firma), el
SDK funciona: es la misma salida que dan JavaScript, Python, Rust y la CLI. Tus propios documentos siguen la forma del
[modelo de entrada](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/input-model.md).

## Enviar a la SUNAT y leer la respuesta

```php
use Kembec\SunatSdk\SunatSdk;
use Kembec\SunatSdk\SigningKey;
use Kembec\SunatSdk\SolCredentials;

$sdk = new SunatSdk('/usr/local/bin/sunat-sdk', 120.0);     // ruta del binario, tiempo máximo por llamada en segundos

$built  = $sdk->build($document);                             // array, objeto o JSON con la forma de docs/input-model.md del repositorio
$signed = $sdk->sign($built->getXml(), new SigningKey('/secure/cert.pem', '/secure/key.pem'));
$result = $sdk->send($built->getFileName(), $signed->getXml(),
    new SolCredentials($rucAndSolUser, $solPassword), $sendBillEndpoint);

if ($result->isFault()) {
    $result->getFault()->getCode();                           // por ejemplo, "1033", con getClass()
} else {
    $result->getCdr()->isAccepted();                          // código de respuesta, notas, clase
}

foreach ($result->getDiagnosis() as $diagnosis) {             // un elemento por código de la respuesta
    $diagnosis->getCode();                                    // "2800"
    $diagnosis->getDetail();                                  // nodo y valor que nombra la SUNAT: cbc:ID/schemeID = "1"
    $diagnosis->getFields()[0]->getInputPath();               // el campo de tu JSON: "customer.document_type"
}

$sdk->status($ruc, '01', 'F001', 123, $credentials, $statusEndpoint);
$sdk->zip($fileName, $xml);                                   // ZipResult
$sdk->cdr($cdrZipOrXmlBytes);                                 // Cdr
```

- `isAccepted()` es verdadero solo con código 0 y sin notas. Con código 0 y cualquier nota, sea cual sea su código, la
  CDR queda `observed`; `isValidDocument()` es verdadero en los dos casos. El 2404 (una nota que modifica un comprobante
  físico) es observación aunque esté por debajo de 4000.
- `getDiagnosis()` (en `send`, `summary`, `ticket` y `status`) dice qué campo de tu entrada causó cada código: el del
  error SOAP, el de la CDR si no es 0 y el de cada nota que empieza con un código (`"4287 - ..."`). Sale de las reglas
  de validación de la SUNAT del 26.08.2026 para 01, 03, 07 y 08; `getCertainty()` es `fuente` cuando la regla apunta a
  un solo campo e `inferido` cuando calcula sobre varios. Es una lista vacía sin veredicto y con un binario anterior
  ([detalle](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/cli.md#diagnosis)).
- `send`, `summary`, `ticket` y `status` esperan a la SUNAT 90 segundos, o el tiempo máximo por llamada menos 5 si es
  más corto. El tercer argumento opcional del constructor cambia esa espera, en segundos enteros, y tiene que dejar esos
  5 de margen.

### Resúmenes diarios (RC) y comunicaciones de baja (RA)

Se arman y firman con `build` y `sign`, como un comprobante. `summary` los envía al mismo `billService` de `send` y
devuelve un ticket; `ticket` pregunta por él hasta que la SUNAT lo procese.

```php
use Kembec\SunatSdk\Result\TicketResult;

$credentials = new SolCredentials($rucAndSolUser, $solPassword);
$built  = $sdk->build($summaryDocument);                      // RC o RA: el nombre es RUC-RC-AAAAMMDD-N o RUC-RA-…
$signed = $sdk->sign($built->getXml(), new SigningKey('/secure/cert.pem', '/secure/key.pem'));
$sent   = $sdk->summary($built->getFileName(), $signed->getXml(), $credentials, $sendBillEndpoint);

if ($sent->isFault()) {
    $sent->getFault()->getCode();                             // por ejemplo, "2223": ese archivo ya se presentó
} else {
    $ticket = $sent->getTicket();                             // guárdalo antes que nada: es lo único que lleva al resultado
    $status = $sdk->ticket($ticket, $credentials, $sendBillEndpoint);
    if ($status->getState() === TicketResult::PROCESSED) {
        $status->getCdr()->isValidDocument();                 // la CDR del resumen o de la baja
    } elseif ($status->isInProcess() || $status->isNoVerdict()) {
        // 98 o sin respuesta: vuelve a consultar más tarde con el mismo ticket
    }                                                         // PROCESSED_WITH_ERRORS (99): mira la CDR o el código
}
```

- Un ticket en proceso (98) o sin veredicto (`isNoVerdict()`, un 401 incluido) se vuelve a consultar con el mismo ticket.
  Nunca se reenvía el resumen: el ticket prueba que la SUNAT lo recibió.
- Un `summary` sin respuesta lanza `SdkErrorException`, porque el resumen pudo haber llegado y no queda ticket: se
  revisan los comprobantes con `validity` y se reenvía el mismo archivo con el mismo nombre; un duplicado se rechaza
  (2223, 2324, 2282).

### Consultar el estado de un comprobante (Consulta Integrada)

`validity` pregunta a la Consulta Integrada de la SUNAT si una factura, boleta o nota (01, 03, 07, 08) existe y en qué
estado está. Usa las «Credenciales de API SUNAT» que se crean en SOL para esta consulta: no el usuario SOL ni el par de
la GRE.

```php
use Kembec\SunatSdk\ApiCredentials;
use Kembec\SunatSdk\Result\ValidityResult;

$check = $sdk->validity('20123456789', '01', 'F001', 123, '2026-10-16', '118.00',
    new ApiCredentials($clientId, $clientSecret));             // fecha de emisión y total del comprobante

if ($check->hasState()) {
    $check->getState();                                       // ValidityResult::ACCEPTED, VOIDED, NOT_REPORTED…
    $check->getObservations();                                // dónde difieren tus datos de los de la SUNAT
} elseif ($check->isQueryError()) {
    $check->getErrorCode();                                   // la SUNAT rechazó la consulta, no el comprobante
} else {
    $check->getStage();                                       // isNoVerdict(): sin respuesta en "token" o "query"
}
```

- **No hay ambiente de pruebas**: la consulta va siempre a producción.
- Ningún resultado es motivo para emitir la operación con otro número: `NOT_REPORTED` no prueba que el comprobante no
  llegó (el ticket de un resumen puede seguir en proceso), así que antes se concilia con `status` o con el ticket.
- Un token o una consulta sin respuesta, un 401 incluido, dan `isNoVerdict()`: se vuelve a consultar con los mismos
  datos.
- Si una factura, boleta o nota enviada sola sigue sin veredicto, en `validity` o en `status`, se reenvía el mismo XML
  firmado con el mismo número mientras el plazo de envío siga abierto, porque la SUNAT lo mide al recibirlo; según sus
  reglas de validación, si ya lo tenía no lo registra dos veces y contesta 1033, 1032 o 0140.

### Guía de remisión remitente (09) por la API REST

`greSend` y `greTicket` llegaron en la 0.1.0-beta.5, con los comandos `gre-send` y `gre-ticket` en su binario.

```php
use Kembec\SunatSdk\ApiCredentials;

$api = new ApiCredentials($greClientId, $greClientSecret);    // el par de GRE de «Credenciales de API SUNAT»
$sol = new SolCredentials($rucAndSolUser, $solPassword);
$sent = $sdk->greSend($built->getFileName(), $signed->getXml(), $api, $sol, $tokenEndpoint, $greEndpoint);

if ($sent->hasTicket()) {
    $ticket = $sent->getTicket();                             // guárdalo antes que nada: es lo único que lleva al resultado
    $status = $sdk->greTicket($ticket, $api, $sol, $tokenEndpoint, $greEndpoint);
    $status->isInProcess();                                   // 98: vuelve a consultar con el mismo ticket
    $status->getCdr();                                        // 0: la CDR leída, como cdr()
    $status->getError();                                      // 99: código, descripción y clase
} elseif ($sent->isApiError()) {
    $sent->getApiError()->getErrors();                        // lo que la SUNAT rechazó, en sus palabras
}
```

- **La SUNAT no tiene ambiente de pruebas para la GRE**, y ninguna guía de este SDK figura como aceptada por la SUNAT.
- Los endpoints no tienen valor por defecto y solo se aceptan `https://` (los del manual:
  `https://api-seguridad.sunat.gob.pe/v1/clientessol` para el token y `https://api-cpe.sunat.gob.pe/v1` para la API).
  Cada llamada pide su propio token, dentro de la misma espera HTTP que `send`.
- Si el envío no recibe respuesta, `greSend` lanza `SdkErrorException`: la guía pudo haber llegado, así que se reenvía el
  mismo archivo, nunca la operación con otro número. Un token o una consulta sin respuesta dan `isNoVerdict()`, como en
  `validity`.

Secretos: las credenciales SOL y de API, los endpoints y las rutas de las llaves llegan al binario solo por su entorno, nunca
por sus argumentos. El proceso hijo no hereda el entorno del padre, y cualquier valor que se le pasa se reemplaza por
`[redacted]` en los mensajes de error. `SolCredentials`, `ApiCredentials` y `SigningKey` ocultan sus valores en `var_dump` y `print_r`.

Errores, todos subclases de `Exception\SunatSdkException`:

| Excepción | Cuándo |
|---|---|
| `SdkErrorException` | el binario terminó con código distinto de cero; el mensaje es su `error`, con `getCommand()` y `getExitCode()` |
| `BinaryNotFoundException` | la ruta no es un archivo |
| `BinaryNotExecutableException` | no tiene permiso de ejecución, o el proceso no pudo arrancar |
| `TimeoutException` | la llamada superó el tiempo máximo; el proceso se termina |
| `InvalidOutputException` | código 0, pero la salida estándar no es el JSON que promete el contrato |
| `InstallException` | `sunat-sdk-install` no pudo descargar, verificar o guardar el binario; `UnsupportedPlatformException` fuera de Linux x86_64 |

Un error SOAP de la SUNAT es un resultado (`isFault()`), no una excepción.

## Qué cubre y qué no

- IGV al 18 % y al 10,5 %, operaciones gravadas, exoneradas, inafectas, de exportación y gratuitas, en soles y en
  dólares, al contado o a crédito en cuotas; descuento por línea; detracción con operación 1001 o 1003; retención del
  IGV informada en el comprobante; IVAP; orden de compra y guías de remisión citadas.
- Lo que el SDK no cubre lo rechaza con un error explícito en vez de armar un XML que la SUNAT rechazaría: ISC, ICBPER,
  percepción, anticipos, descuentos y cargos globales, entre otros.
- En la guía de remisión quedan fuera los traslados de importación y exportación y la guía del transportista (31).

La lista completa está en el
[alcance del SDK](https://github.com/Kembec/facturacion-electronica-sunat-sdk#alcance) y cada campo, en el
[modelo de entrada](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/input-model.md).

## Pruebas

Lo que aceptó la SUNAT y cómo se comprueba cada versión está en
[Cómo se comprueba](https://github.com/Kembec/facturacion-electronica-sunat-sdk#cómo-se-comprueba): la beta de la SUNAT
aceptó 36 de los 38 documentos de ejemplo; las 2 guías de remisión no tienen beta.

Para correr las pruebas de este paquete, desde la raíz del repositorio (donde está su único `composer.json`):

```bash
curl -sSL https://getcomposer.org/download/latest-stable/composer.phar -o composer.phar
php composer.phar install && vendor/bin/phpunit
```

Corren contra el binario real (`SUNAT_SDK_BIN`, o `target/debug/sunat-sdk` por defecto) y no envían nada a la SUNAT:
cada uno de los 38 documentos de
[fixtures](https://github.com/Kembec/facturacion-electronica-sunat-sdk/tree/main/fixtures) tiene que dar, con `build`
y `sign`, el XML y la firma exactos de
[tests/vectors](https://github.com/Kembec/facturacion-electronica-sunat-sdk/tree/main/tests/vectors); `summary` y
`ticket` se prueban contra la salida del núcleo para cada respuesta de la SUNAT (ticket, rechazo, 0, 98, 99 y sin
respuesta), y `validity`, `greSend` y `greTicket`, contra respuestas grabadas de los manuales de la SUNAT y, en la GRE,
de un cliente de terceros. El CI las corre en cada cambio en PHP 8.1.

## Licencia

MIT, © 2026 Manuel Benancio: ver
[`LICENSE`](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/LICENSE).
