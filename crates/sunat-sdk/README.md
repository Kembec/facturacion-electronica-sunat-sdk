# kembec-sunat-sdk

Cada llamada a la SUNAT como una entrada JSON y una salida JSON, sin hacer red: arma la petición, tú la envías con tu
cliente HTTP, y lee lo que volvió con el mismo resultado que imprime la línea de comandos del SDK. Cubre el envío de
comprobantes, resúmenes y bajas por SOAP (`send`, `summary`, `ticket`, `status`), la consulta de validez de un
comprobante y el envío de la guía de remisión remitente por la API REST. Es parte del
[SDK de facturación electrónica SUNAT](https://github.com/Kembec/facturacion-electronica-sunat-sdk) para Rust.

SUNAT (Peru) e-invoicing calls without I/O.

Cada respuesta sale clasificada (`accepted`, `observed`, `rejected`, `exception`, `already_registered`…) junto al
código de la SUNAT. Cuando una consulta no recibe respuesta, el resultado es `no_verdict` y se vuelve a preguntar; un envío sin
respuesta es un error, porque el documento pudo haber llegado. El detalle de cada salida está en la
[referencia de la CLI](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/cli.md).

## Instalación

```bash
cargo add kembec-sunat-sdk@0.1.0-beta.4 serde_json
```

## Enviar una factura firmada

`post` es tu cliente HTTP: recibe el `SOAPAction` y el sobre, y devuelve el estado HTTP y el cuerpo.

```rust,no_run
use serde_json::{json, Value};
use kembec_sunat_sdk::exchange::{self, Exchange, Reply};

fn send(
    name: &str,           // 20123456789-01-F001-1
    signed_xml: &str,     // el XML firmado con kembec-sunat-xmldsig
    sol_user: &str,       // RUC + usuario SOL
    sol_password: &str,
    post: impl Fn(&str, &str) -> (u16, Vec<u8>),
) -> Result<Value, String> {
    let input = json!({ "name": name, "xml": signed_xml });
    let envelope = exchange::request(Exchange::Send, &input, sol_user, sol_password)?;
    let (status, body) = post(Exchange::Send.soap_action(), &envelope);
    // {"outcome": "cdr", "cdr": {...}, ...} o {"outcome": "fault", "code", "message", "class"}
    exchange::read(Exchange::Send, &input, Reply::from_http(status, &body))
}
```

## Los crates del SDK

| Crate | Qué hace |
|---|---|
| [`kembec-sunat-model`](https://crates.io/crates/kembec-sunat-model) | lee el JSON del documento y comprueba que esté dentro del alcance del SDK |
| [`kembec-sunat-ubl`](https://crates.io/crates/kembec-sunat-ubl) | arma el XML UBL en forma canónica |
| [`kembec-sunat-xmldsig`](https://crates.io/crates/kembec-sunat-xmldsig) | lo firma con tu certificado y verifica firmas |
| [`kembec-sunat-transport`](https://crates.io/crates/kembec-sunat-transport) | el ZIP, los sobres SOAP y la lectura de respuestas y CDR |
| [`kembec-sunat-sdk`](https://crates.io/crates/kembec-sunat-sdk) | cada llamada a la SUNAT como JSON: arma la petición y lee la respuesta |

Ninguno abre conexiones ni lee archivos: el POST HTTP lo haces tú, con TLS verificado. Se publican juntos y cada uno
pide a los otros en su versión exacta, así que agrégalos todos con la misma versión.

Cada versión pasa, compilada desde los paquetes que se suben a crates.io, la batería de vectores de compatibilidad del
[repositorio](https://github.com/Kembec/facturacion-electronica-sunat-sdk): el mismo `DigestValue` y el mismo
`SignatureValue` que dan JavaScript, Python, PHP y la línea de comandos para los mismos datos.

Licencia MIT ([LICENSE](LICENSE)). Este proyecto no está afiliado a la SUNAT ([NOTICE](NOTICE)).
