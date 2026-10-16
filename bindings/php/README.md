# kembec/facturacion-electronica-sunat-sdk (PHP)

Facturación electrónica de la SUNAT (Perú) desde **PHP**: arma, firma, comprime y envía facturas (01), boletas (03),
notas de crédito (07) y notas de débito (08), consulta su CDR y la lee; arma y firma además resúmenes diarios (RC) y
comunicaciones de baja (RA). Es el cliente PHP del binario `sunat-sdk`, el núcleo en Rust del
[SDK](https://github.com/Kembec/facturacion-electronica-sunat-sdk) que usan también sus versiones para JavaScript y
TypeScript, Python y Rust, así que el mismo documento da la misma firma en todas. Cada llamada ejecuta el binario una
vez con `proc_open`; el contrato de cada comando está en el
[contrato de la CLI](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/cli.md).

SUNAT (Peru) e-invoicing for PHP: UBL XML, XML-DSig signature, SOAP web service and CDR.

Requiere PHP 7.4 o posterior con la extensión `json`; el instalador del binario usa además `openssl`. Tu sistema
calcula los importes; el SDK los valida y los escribe, no calcula impuestos. Guías y ejemplos: https://fe.kembec.com

## Instalación

```bash
composer require kembec/facturacion-electronica-sunat-sdk:^0.1@beta
export "$(vendor/bin/sunat-sdk-install)"     # opcional: --dir=<directorio>
```

Las versiones publicadas son beta, por eso la restricción `^0.1@beta`. `sunat-sdk-install` descarga, del release de
GitHub `v<SunatSdk::VERSION>`, el binario estático para Linux x86_64 y su `.sha256`, con TLS verificado. Deja el binario
ejecutable solo si el SHA-256 coincide e imprime `SUNAT_SDK_BIN=<ruta absoluta>`: esa ruta es el primer argumento de
`SunatSdk`. Por defecto lo deja en `vendor/kembec/facturacion-electronica-sunat-sdk/bindings/php/bin/`; usa `--dir` si
`vendor/` es de solo lectura al ejecutar. Composer no lo corre por su cuenta. En otra plataforma falla e indica cómo
compilar el binario con Rust:

```bash
git clone https://github.com/Kembec/facturacion-electronica-sunat-sdk.git
cargo build --manifest-path facturacion-electronica-sunat-sdk/Cargo.toml -p sunat-sdk-cli --release --locked
export SUNAT_SDK_BIN="$PWD/facturacion-electronica-sunat-sdk/target/release/sunat-sdk"
```

## Tu primer comprobante

Baja a la carpeta de tu proyecto la factura de prueba y el certificado de prueba del repositorio. Ese certificado es
autogenerado y no sirve para emitir.

```bash
base=https://raw.githubusercontent.com/Kembec/facturacion-electronica-sunat-sdk/v0.1.0-beta.2
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

`+te61a2ub5MlTFf8tKTPNGWEXKI=` es el `DigestValue` del vector
[`01-invoice-taxed-cash`](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/tests/vectors/01-invoice-taxed-cash.signed.xml):
el mismo que dan JavaScript, Python, Rust y la CLI.

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

$sdk->status($ruc, '01', 'F001', 123, $credentials, $statusEndpoint);
$sdk->zip($fileName, $xml);                                   // ZipResult
$sdk->cdr($cdrZipOrXmlBytes);                                 // Cdr
```

- `isAccepted()` es verdadero solo con código 0 y sin observaciones. Con código 0 y una nota de 4000 o más, la CDR
  queda `observed`; `isValidDocument()` es verdadero en los dos casos. La clase `observed` e `isValidDocument()` llegan
  en la versión siguiente a 0.1.0-beta.2.
- `send` y `status` esperan a la SUNAT hasta 90 segundos, o menos si el tiempo máximo por llamada no deja 5 segundos
  de margen. El tercer argumento opcional del constructor fija esa espera, en segundos enteros.
- `build` y `sign` también arman y firman resúmenes diarios (RC) y comunicaciones de baja (RA), con el binario de la
  versión siguiente a 0.1.0-beta.2. Este cliente no los envía ni consulta su ticket: para eso están los comandos
  `summary` y `ticket` de la CLI.

Secretos: las credenciales SOL, los endpoints y las rutas de las llaves llegan al binario solo por su entorno, nunca
por sus argumentos. El proceso hijo no hereda el entorno del padre, y cualquier valor que se le pasa se reemplaza por
`[redacted]` en los mensajes de error. `SolCredentials` y `SigningKey` ocultan sus valores en `var_dump` y `print_r`.

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

## Cómo se comprueba

Desde la raíz del repositorio, donde está el único `composer.json` del paquete:

```bash
curl -sSL https://getcomposer.org/download/latest-stable/composer.phar -o composer.phar
php composer.phar install && vendor/bin/phpunit
```

Corren contra el binario real: `SUNAT_SDK_BIN`, o `target/debug/sunat-sdk` por defecto. Las pruebas del instalador
sirven un release desde un `php -S` local y no necesitan red. Los 22 casos de
[fixtures](https://github.com/Kembec/facturacion-electronica-sunat-sdk/tree/main/fixtures) (comprobantes,
resúmenes diarios y comunicaciones de baja) tienen que dar, con `build` y `sign`, el XML de
[tests/vectors](https://github.com/Kembec/facturacion-electronica-sunat-sdk/tree/main/tests/vectors) en C14N
inclusiva y exactamente su `DigestValue` y su `SignatureValue`, firmados con el certificado de prueba autogenerado.
No se envía nada a la SUNAT. El CI las corre en cada cambio en PHP 8.1 contra el binario compilado; esos mismos documentos
firmados, enviados con la CLI, dieron código 0 en la beta de la SUNAT el 16/10/2026.

## Licencia

MIT, © 2026 Manuel Benancio: ver
[`LICENSE`](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/LICENSE).
