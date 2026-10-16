# SDK de facturación electrónica SUNAT (Perú) para Python

Un SDK para emitir comprobantes electrónicos ante la SUNAT desde **Python**. Con los datos de una factura (01), una
boleta (03), una nota de crédito (07) o de débito (08), un resumen diario de boletas (RC) o una comunicación de baja
(RA), arma el archivo XML que pide la SUNAT, lo firma con tu certificado digital, lo comprime, lo envía y lee la
respuesta de la SUNAT, que dice si lo aceptó. Es la versión para Python del
[SDK](https://github.com/Kembec/facturacion-electronica-sunat-sdk), que también funciona en JavaScript y TypeScript,
PHP, Rust y la línea de comandos: los mismos datos dan el mismo comprobante firmado en todos.

SUNAT (Peru) electronic invoicing SDK for Python.

Tu sistema calcula los importes; el paquete los valida y los pone en el XML, pero no calcula impuestos. Guías y
ejemplos: https://fe.kembec.com

## Instalación

El paquete está en PyPI desde la 0.1.0-beta.3. Como es una beta, `pip` la instala solo si se le pide:

```bash
pip install --pre facturacion-electronica-sunat-sdk
```

El paquete trae el núcleo en Rust del SDK ya compilado (wheels abi3), así que no llama a ningún binario externo. Hay
wheels para CPython 3.10 o posterior en Linux (x86_64 y aarch64, glibc), macOS (arm64 y x86_64) y Windows (x86_64). En
otra plataforma, pip lo compila desde el código fuente (el sdist) y necesita Rust.

Para instalar la versión en desarrollo (la rama `main`) desde el repositorio hace falta Rust:

```bash
git clone https://github.com/Kembec/facturacion-electronica-sunat-sdk.git
cd facturacion-electronica-sunat-sdk
cp LICENSE NOTICE bindings/python/
pip install ./bindings/python
```

## Tu primer comprobante

Baja a una carpeta vacía la factura de prueba y el certificado de prueba del repositorio. Ese certificado es
autogenerado y no sirve para emitir.

```bash
base=https://raw.githubusercontent.com/Kembec/facturacion-electronica-sunat-sdk/v0.1.0-beta.3
curl -fsSLO "$base/fixtures/01-invoice-taxed-cash.json"
curl -fsSLO "$base/tests/vectors/test-cert/test-cert.pem"
curl -fsSLO "$base/tests/vectors/test-cert/test-key.pem"
```

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

`+te61a2ub5MlTFf8tKTPNGWEXKI=` es el `DigestValue` del vector
[`01-invoice-taxed-cash`](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/tests/vectors/01-invoice-taxed-cash.signed.xml):
el mismo que dan JavaScript, PHP, Rust y la CLI. Tus documentos llevan la forma del
[modelo de entrada](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/input-model.md), como
`dict` o como texto JSON.

## Enviar a la SUNAT

```python
sent = sunat_sdk.exchange(
    "send", {"name": built["file_name"], "xml": signed["xml"]},
    sol_user, sol_password, bill_service_url,            # sol_user: RUC + usuario SOL
)
# {"outcome": "cdr", "cdr": {...}, "cdr_zip_base64": "..."} o {"outcome": "fault", "code", "message", "class"}
```

## API

`build` arma XML UBL 2.1 (comprobantes) o UBL 2.0 (resúmenes y bajas), y `sign` lo firma con XML-DSig.

- `build(document)`, `sign(xml, cert_pem, key_pem, signature_id=None)`, `zip({name, xml})` y `cdr({cdr_base64})`
  devuelven el mismo JSON que la CLI y el paquete npm, ya convertido en `dict`. La llave PEM tiene que ser RSA sin
  cifrar; `signature_id` es `signatureKG` si no se indica.
- `exchange(kind, input, user, password, endpoint, timeout=60)`: `kind` es `send`, `summary`, `ticket` o `status`, e
  `input`, la entrada de ese comando de la CLI: `name` y `xml` en `send` y `summary`; `ticket` en `ticket`; `ruc`,
  `document_type`, `series` y `number` en `status`. `send`, `summary` y `ticket` van a la URL del billService, y `status`, a la
  del billConsultService. Verifica TLS con el contexto por defecto del sistema; `context=` admite otro
  `ssl.SSLContext`, nunca uno sin verificación.
- `soap_request`, `soap_response` y `soap_timeout` son las mismas piezas sin red, para quien hace el POST con su propio
  cliente HTTP (`httpx`, `aiohttp`).
- Un error SOAP de la SUNAT es un resultado (`{"outcome": "fault", ...}`), no una excepción. Si `ticket` o `status` no
  reciben respuesta, el resultado es `{"outcome": "no_verdict", ...}`: se vuelve a consultar y nunca se reenvía. En
  `send` y `summary`, ese caso lanza `SunatSdkError`, porque el documento pudo haber llegado.
- Toda entrada inválida lanza `sunat_sdk.SunatSdkError` (subclase de `ValueError`) con el motivo.
- El certificado, la llave y las credenciales SOL los pasa quien llama: el paquete no los busca en archivos ni en el
  entorno, y no escribe logs.

## Cómo se comprueba

En cada cambio, el CI instala el paquete en un entorno limpio y exige, para los 26 vectores de prueba del repositorio, el
XML sin firmar igual al del vector en C14N y el `DigestValue` y el `SignatureValue` exactos: las mismas firmas que dieron
código 0 en la beta de la SUNAT el 16/10/2026, enviadas con la CLI. `exchange` se prueba contra un servidor local, sin
red.

## Licencia

MIT, © 2026 Manuel Benancio: ver
[`LICENSE`](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/LICENSE).
