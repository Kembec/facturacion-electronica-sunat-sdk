# facturacion-electronica-sunat-sdk (Python)

Facturación electrónica de la SUNAT (Perú) desde **Python**: arma, firma, comprime y envía a la SUNAT facturas (01),
boletas (03), notas de crédito (07) y notas de débito (08) en UBL 2.1, y resúmenes diarios (RC) y comunicaciones de baja
(RA) en UBL 2.0, y lee su respuesta. Usa el mismo núcleo en Rust que las versiones del
[SDK](https://github.com/Kembec/facturacion-electronica-sunat-sdk) para JavaScript y TypeScript, PHP, Rust y la línea
de comandos, compilado dentro de la wheel: no llama a ningún binario externo, y el mismo documento da la misma firma en
todas.

SUNAT (Peru) e-invoicing for Python: UBL XML, XML-DSig signature, SOAP web service and CDR.

Tu sistema calcula los importes; el paquete los valida y los escribe, no calcula impuestos. Guías y ejemplos:
https://fe.kembec.com

## Instalación

El paquete se publica en PyPI desde la versión siguiente a 0.1.0-beta.2, la primera que lo incluye. Como esa versión es
beta, `pip` la instala solo si se le pide:

```bash
pip install --pre facturacion-electronica-sunat-sdk
```

Wheels abi3 para CPython 3.10 o posterior en Linux (x86_64 y aarch64, glibc), macOS (arm64 y x86_64) y Windows
(x86_64). En otra plataforma, pip compila el sdist y necesita Rust.

Hasta que esté en PyPI, se instala desde el repositorio, con Rust:

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
base=https://raw.githubusercontent.com/Kembec/facturacion-electronica-sunat-sdk/v0.1.0-beta.2
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

En cada cambio, el CI instala el paquete en un entorno limpio y exige, para los 22 vectores de prueba del repositorio, el
XML sin firmar igual al del vector en C14N y el `DigestValue` y el `SignatureValue` exactos: las mismas firmas que dieron
código 0 en la beta de la SUNAT el 16/10/2026, enviadas con la CLI. `exchange` se prueba contra un servidor local, sin
red.

## Licencia

MIT, © 2026 Manuel Benancio: ver
[`LICENSE`](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/LICENSE).
