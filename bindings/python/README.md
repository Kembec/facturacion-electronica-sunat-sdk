# SDK de facturación electrónica SUNAT (Perú) para Python

Un SDK para emitir comprobantes electrónicos ante la SUNAT desde **Python**, para quien programa el sistema de
facturación de una empresa. Con los datos de una factura (01), una boleta (03), una nota de crédito (07) o de débito
(08), un resumen diario de boletas (RC) o una comunicación de baja (RA):

- arma el archivo XML que pide la SUNAT y lo firma con tu certificado digital;
- lo comprime, lo envía y lee la respuesta de la SUNAT, que dice si lo aceptó;
- después consulta el estado de un comprobante ya enviado en la Consulta Integrada de la SUNAT (`validity`: aceptado,
  anulado o no informado).

También arma, firma y envía la guía de remisión remitente (09) por la API REST de la SUNAT
([abajo](#guía-de-remisión-remitente-09-por-la-api-rest)). Es la versión para Python del
[SDK](https://github.com/Kembec/facturacion-electronica-sunat-sdk), que también funciona en JavaScript y TypeScript,
PHP, Rust y la línea de comandos: los mismos datos dan el mismo comprobante firmado en todos.

SUNAT (Peru) electronic invoicing SDK for Python.

Tu sistema calcula los importes; el paquete los valida y los pone en el XML, pero no calcula impuestos. Guías y
ejemplos: https://fe.kembec.com

## Instalación

La versión publicada es una beta:

```bash
pip install --pre facturacion-electronica-sunat-sdk
```

El paquete trae el núcleo en Rust del SDK ya compilado (wheels abi3), así que no llama a ningún binario externo. Hay
wheels para CPython 3.10 o posterior en Linux (x86_64 y aarch64, glibc), macOS (arm64 y x86_64) y Windows (x86_64). En
otra plataforma, pip lo compila desde el sdist y necesita Rust.

Para instalar `main` desde el repositorio hace falta Rust:

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
base=https://raw.githubusercontent.com/Kembec/facturacion-electronica-sunat-sdk/v1.0.0-rc.1
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

Si imprime `20123456789-01-F001-1 +te61a2ub5MlTFf8tKTPNGWEXKI=` (el nombre del archivo y la huella de la firma), el
paquete funciona: es la misma salida que dan JavaScript, PHP, Rust y la CLI. Tus propios documentos siguen la forma del
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

## Consultar el estado de un comprobante (Consulta Integrada)

`validity` pregunta a la Consulta Integrada de la SUNAT si una factura, boleta o nota (01, 03, 07, 08) existe y en qué
estado está. Usa las «Credenciales de API SUNAT» que se crean en SOL para esta consulta: no el usuario SOL ni el par de
la GRE.

```python
check = sunat_sdk.validity(
    {"ruc": "20123456789", "document_type": "01", "series": "F001", "number": 123,
     "issue_date": "2026-10-16", "total": "118.00"},       # fecha de emisión y total del comprobante
    client_id, client_secret,
)
# {"outcome": "validity", "state": "accepted" | "voided" | "not_reported" | ..., "observations": [...], ...}
# {"outcome": "query_error", "error_code", ...}: la SUNAT rechazó la consulta, no el comprobante
# {"outcome": "no_verdict", "stage": "token" | "query", ...}: sin respuesta (401 incluido), vuelve a consultar
```

- **No hay ambiente de pruebas**: por defecto la consulta va a producción; `token_base` y `base` reemplazan las URL
  base. El token y el secreto nunca vuelven en el resultado.
- Ningún resultado es motivo para emitir la operación con otro número: `not_reported` no prueba que el comprobante no
  llegó (el ticket de un resumen puede seguir en proceso), así que antes se concilia con `ticket` o `status`.

## Guía de remisión remitente (09) por la API REST

`gre_send`, `gre_ticket` y sus piezas sin red llegaron en la 0.1.0-beta.5.

```python
sent = sunat_sdk.gre_send(
    {"name": built["file_name"], "xml": signed["xml"]},  # RUC-09-SERIE-NUMERO, ya firmada
    client_id, client_secret,                            # el par de GRE de «Credenciales de API SUNAT»
    sol_user, sol_password,                              # sol_user: RUC + usuario SOL
    token_base, api_base,                                # sin valor por defecto
)
# {"outcome": "ticket", "ticket", "received_at", "zip_name", "hash_zip"}: guarda el ticket antes que nada
status = sunat_sdk.gre_ticket(sent["ticket"], client_id, client_secret, sol_user, sol_password, token_base, api_base)
# state: "processed" (0, con la CDR leída), "in_process" (98) o "processed_with_errors" (99, con su error)
```

**La SUNAT no tiene ambiente de pruebas para la GRE**, y ninguna guía de este SDK figura como aceptada por la SUNAT. Las
URL base siempre las pasas tú (las del manual: `https://api-seguridad.sunat.gob.pe/v1/clientessol` para el token y
`https://api-cpe.sunat.gob.pe/v1` para la API). Cada llamada pide su propio token y `timeout` cubre las dos.

- Con 98, vuelve a consultar con el mismo ticket; nunca reenvíes la guía por eso.
- Si el envío no recibe respuesta, `gre_send` lanza `SunatSdkError`: la guía pudo haber llegado, así que se reenvía el
  mismo archivo, nunca la operación con otro número. Un token o una consulta sin respuesta dan
  `{"outcome": "no_verdict", "stage", ...}`, y un rechazo de la SUNAT, `{"outcome": "api_error", "stage", "errors", ...}`.
- Para usar tu propio cliente HTTP, o reutilizar el token hasta diez minutos antes de que venza, están las mismas piezas
  sin red: `gre_token_request` y `gre_token_response`, `gre_send_request` y `gre_send_response`, `gre_ticket_request` y
  `gre_ticket_response`, más `gre_token_usable` y `gre_timeout`.

## API

`build` arma XML UBL 2.1 (comprobantes) o UBL 2.0 (resúmenes y bajas), y `sign` lo firma con XML-DSig.

- `build(document)`, `sign(xml, cert_pem, key_pem, signature_id=None)`, `zip({name, xml})` y `cdr({cdr_base64})`
  devuelven el mismo JSON que la CLI y el paquete npm, ya convertido en `dict`. La llave PEM tiene que ser RSA sin
  cifrar; `signature_id` es `signatureKG` si no se indica.
- `exchange(kind, input, user, password, endpoint, timeout=60)` hace una llamada al web service de la SUNAT. `input` es
  la entrada del mismo comando de la CLI:

  | `kind` | `input` | `endpoint` |
  |---|---|---|
  | `send` | `name`, `xml` | billService |
  | `summary` | `name`, `xml` | billService |
  | `ticket` | `ticket` | billService |
  | `status` | `ruc`, `document_type`, `series`, `number` | billConsultService |

  Verifica TLS con el contexto por defecto del sistema; `context=` admite otro `ssl.SSLContext`, nunca uno sin
  verificación.
- `soap_request`, `soap_response` y `soap_timeout` son las mismas piezas sin red, para quien hace el POST con su propio
  cliente HTTP (`httpx`, `aiohttp`).
- Un envío rechazado u observado trae `diagnosis` en su resultado (`soap_response`, `soap_timeout` y `exchange`): por
  cada código de la respuesta, el campo de la entrada que lo causó, p. ej. `customer.document_type` para el 2800
  de `cbc:ID/schemeID`. `diagnose(document_type, code, message)` lo da para un solo código.
- `validity(input, client_id, client_secret, timeout=60, context=None, token_base=None, base=None)` pide el token y hace
  la consulta dentro de `timeout`. `validity_token_request`, `validity_token_response`, `validity_request`,
  `validity_response` y `validity_timeout` son sus piezas sin red.
- Un error SOAP de la SUNAT es un resultado (`{"outcome": "fault", ...}`), no una excepción. Si `ticket` o `status` no
  reciben respuesta, el resultado es `{"outcome": "no_verdict", ...}`: se vuelve a consultar. Con `ticket`, el resumen
  nunca se reenvía, porque el ticket prueba que llegó. Si `status` sigue sin veredicto, se reenvía el mismo XML firmado
  con el mismo número mientras el plazo de envío siga abierto, porque la SUNAT lo mide al recibirlo; según sus reglas
  de validación, si ya lo tenía no lo registra dos veces y contesta 1033, 1032 o 0140. En `send` y `summary`, ese caso
  lanza `SunatSdkError`, porque el documento pudo haber llegado. Un `summary` sin respuesta no deja ticket: se revisan
  los comprobantes con `validity` y se reenvía el mismo archivo con el mismo nombre.
- Toda entrada inválida lanza `sunat_sdk.SunatSdkError` (subclase de `ValueError`) con el motivo.
- El certificado, la llave y las credenciales SOL los pasa quien llama: el paquete no los busca en archivos ni en el
  entorno, y no escribe logs.

## Qué cubre y qué no

- IGV al 18 % y al 10,5 %, operaciones gravadas, exoneradas, inafectas, de exportación y gratuitas, en soles y en
  dólares, al contado o a crédito en cuotas; descuento por línea; detracción con operación 1001 o 1003; retención del
  IGV informada en el comprobante; IVAP; orden de compra y guías de remisión citadas.
- Lo que el SDK no cubre lo rechaza con `SunatSdkError` en vez de armar un XML que la SUNAT rechazaría: ISC, ICBPER,
  percepción, anticipos, descuentos y cargos globales, entre otros.
- En la guía de remisión quedan fuera los traslados de importación y exportación y la guía del transportista (31).

La lista completa está en el
[alcance del SDK](https://github.com/Kembec/facturacion-electronica-sunat-sdk#alcance) y cada campo, en el
[modelo de entrada](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/input-model.md).

## Pruebas

Lo que aceptó la SUNAT y cómo se comprueba cada versión está en
[Cómo se comprueba](https://github.com/Kembec/facturacion-electronica-sunat-sdk#cómo-se-comprueba): la beta de la SUNAT
aceptó 36 de los 38 documentos de ejemplo; las 2 guías de remisión no tienen beta. Para este paquete, el CI lo instala
en un entorno limpio y exige el XML y la firma exactos de los 38; `exchange`, `validity`, `gre_send` y `gre_ticket` se
prueban contra un servidor local, sin red; las tres últimas, con respuestas grabadas de los manuales de la SUNAT y, en
la GRE, de un cliente de terceros.

## Licencia

MIT, © 2026 Manuel Benancio: ver
[`LICENSE`](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/LICENSE).
