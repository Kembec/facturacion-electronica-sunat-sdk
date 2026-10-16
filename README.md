# facturacion-electronica-sunat-sdk

Facturación electrónica de la SUNAT (Perú) desde PHP, JavaScript o la línea de comandos, con un mismo núcleo en Rust:
los tres producen el mismo XML y la misma firma.

**Estado:** genera y firma facturas, boletas, notas de crédito y notas de débito, y en cada cambio el CI exige, en los
tres, la firma exacta de 12 vectores de prueba. La aceptación en la beta de la SUNAT todavía no está comprobada, y los
paquetes de Composer y npm aún no están publicados.

Guías y los 12 ejemplos, con su entrada, su XML y su firma: **https://fe.kembec.com**

## Qué hace hoy

| Capacidad | Estado | Cómo se comprueba |
|---|---|---|
| Factura (01), boleta (03), nota de crédito (07) y nota de débito (08): XML UBL 2.1 y firma | Comprobado en CI | En cada cambio, el CI genera y firma los 12 casos de `fixtures/` y exige el XML de `tests/vectors/` en forma canónica, y exactamente su `DigestValue` y su `SignatureValue`. Lo hace por la CLI, por PHP y por el paquete npm en Node 18, Node 22 y Chrome. Un verificador en Python, que no comparte código con el núcleo, acepta las firmas. |
| ZIP y lectura de la CDR | Comprobado en CI, sin red | Pruebas con CDR sintéticas, escritas según los ejemplos del manual de la SUNAT. |
| Envío (`send`) y consulta de la CDR (`status`) | Implementado, sin probar contra la SUNAT | Los sobres SOAP siguen los ejemplos del manual. Su aceptación en la beta todavía no está comprobada. |
| Resumen diario (RC) y comunicación de baja (RA) | Solo el contrato | Modelo de entrada y vectores firmados. `build` todavía no los genera. |
| Guía de remisión, retención y percepción | No incluidos | — |

Un tipo de documento se da por incorporado cuando cumple tres criterios: su XML coincide con los vectores, su firma da
los mismos valores y la acepta un verificador independiente, y la beta de la SUNAT lo acepta. Los cuatro tipos de la
primera fila cumplen los dos primeros; el tercero falta.

**Solo IGV, y sin cálculo de impuestos.** Los importes los calcula tu sistema y llegan como texto (`"118.00"`): el SDK
los valida y los escribe. Rechaza con un error explícito ISC, ICBPER, IVAP y otros tributos, descuentos o cargos
globales, anticipos, detracción, percepción y referencias a guías.

## Instalación

> **Composer y npm, todavía no.** Los paquetes están preparados, pero no están en Packagist ni en npm, y aún no hay un
> release con el binario: hoy `composer require`, `npm install` y `vendor/bin/sunat-sdk-install` fallan. Cada sección
> muestra cómo se instalará y cómo usarlo hoy desde este repositorio.

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
JSON por la salida; `send` y `status` toman la URL y las credenciales SOL de opciones o del entorno. El contrato
completo está en [`docs/cli.md`](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/cli.md), y el
modelo de entrada, en
[`docs/input-model.md`](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/input-model.md).

Cuando haya releases, cada uno traerá el binario estático para Linux x86_64 y su SHA-256.

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

### JavaScript (WebAssembly)

`build` y `sign` en memoria, con la misma API en Node 18 o posterior y en el navegador. Este paquete no envía a la
SUNAT: el envío está en la CLI y en PHP.

Cuando el paquete esté publicado:

```bash
npm install @kembec/facturacion-electronica-sunat-sdk
```

```js
import { load } from '@kembec/facturacion-electronica-sunat-sdk';

const sdk = await load();
const { file_name, xml } = JSON.parse(sdk.build(documentJson));
const signed = JSON.parse(sdk.sign(xml, certPem, keyPem)); // { xml, digest_value, signature_value }
```

Las dos funciones devuelven texto JSON y lanzan un `Error` con el motivo cuando la entrada no sirve. `load` también
acepta el módulo WebAssembly ya compilado, que es lo que piden entornos como Cloudflare Workers. El CI lo prueba así en
Node, pero no en Workers ni con bundlers; esos casos están descritos en el
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
npm install ../facturacion-electronica-sunat-sdk/bindings/wasm/npm/kembec-facturacion-electronica-sunat-sdk-0.1.0.tgz
```

## Derechos y licencia

- Código de este repositorio: licencia MIT, © 2026 Manuel Benancio (ver [`LICENSE`](LICENSE)).
- La documentación técnica, los catálogos y los esquemas de la SUNAT le pertenecen a la SUNAT. Este repositorio no los
  redistribuye.
- Proyecto independiente: no está afiliado a la SUNAT, y no es un PSE ni un OSE. Quien lo usa sigue siendo el emisor
  electrónico y responde por sus comprobantes.
- Se entrega sin garantía, según la licencia MIT.
