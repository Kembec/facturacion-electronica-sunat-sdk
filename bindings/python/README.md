# facturacion-electronica-sunat-sdk (Python)

Arma, firma, comprime y envía a la SUNAT (Perú) los comprobantes electrónicos desde Python: facturas (01), boletas
(03), notas de crédito (07) y notas de débito (08) en UBL 2.1, y resúmenes diarios (RC) y comunicaciones de baja (RA)
en UBL 2.0. Usa el mismo núcleo en Rust que la CLI, el cliente PHP y el paquete npm del
[repositorio](https://github.com/Kembec/facturacion-electronica-sunat-sdk), compilado dentro de la wheel: no llama a
ningún binario externo.

**Estado:** versión 0.1.0b2 (0.1.0-beta.2 del repositorio), todavía no publicada en PyPI. En cada cambio, el CI instala
la wheel de cada plataforma en un entorno limpio y exige, para los 20 vectores de prueba, el XML sin firmar igual al
vector en C14N y el `DigestValue` y el `SignatureValue` exactos. El envío (`exchange`) lo prueban las pruebas contra un
servidor local, sin red; todavía no se ha enviado nada a la SUNAT desde este paquete. Guías: https://fe.kembec.com

```bash
pip install facturacion-electronica-sunat-sdk
```

Wheels abi3 para CPython 3.10 o posterior en Linux (x86_64 y aarch64, glibc), macOS (arm64 y x86_64) y Windows
(x86_64). En otra plataforma, pip compila el sdist y necesita Rust.

```python
import sunat_sdk

built = sunat_sdk.build(document)                       # dict o JSON con la forma de docs/input-model.md
signed = sunat_sdk.sign(built["xml"], cert_pem, key_pem)  # {xml, digest_value, signature_value}

sent = sunat_sdk.exchange(
    "send", {"name": built["file_name"], "xml": signed["xml"]},
    sol_user, sol_password, bill_service_url,            # sol_user: RUC + usuario SOL
)
# {"outcome": "cdr", "cdr": {...}, "cdr_zip_base64": "..."} o {"outcome": "fault", "code", "message", "class"}
```

- `build(document)`, `sign(xml, cert_pem, key_pem, signature_id=None)`, `zip({name, xml})` y `cdr({cdr_base64})`
  devuelven el mismo JSON que la CLI y el paquete npm, ya convertido en `dict`. La llave PEM tiene que ser RSA sin
  cifrar; `signature_id` es `signatureKG` si no se indica.
- `exchange(kind, input, user, password, endpoint, timeout=60)`: `kind` es `send`, `summary`, `ticket` o `status`, e
  `input`, la entrada de ese comando de la CLI (`send` y `summary`, `{name, xml}`; `ticket`, `{ticket}`; `status`,
  `{ruc, document_type, series, number}`). `send`, `summary` y `ticket` van a la URL del billService, y `status`, a la
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

## Licencia

MIT, © 2026 Manuel Benancio: ver
[`LICENSE`](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/LICENSE).
