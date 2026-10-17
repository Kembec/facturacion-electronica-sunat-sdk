# sunat-transport

Lo que viaja entre tu sistema y los servicios web de la SUNAT, sin hacer red: comprime el XML firmado como lo espera la
SUNAT, arma los sobres SOAP de `sendBill`, `sendSummary`, `getStatus` y `getStatusCdr` con usuario y clave SOL, y lee lo
que la SUNAT contesta, incluidos los SOAP Fault y la constancia de recepción (CDR). Es parte del
[SDK de facturación electrónica SUNAT](https://github.com/Kembec/facturacion-electronica-sunat-sdk) para Rust.

Sans-IO SOAP envelopes, ZIP packaging and CDR parsing for the SUNAT (Peru) web services.

Si prefieres no armar cada paso, `sunat-sdk` hace lo mismo con una entrada y una salida JSON por llamada.

## Instalación

```bash
cargo add sunat-transport@0.1.0-beta.4
```

## Enviar una factura

`post` es tu cliente HTTP: recibe el `SOAPAction` y el sobre, y devuelve el cuerpo de la respuesta. Las faltas SOAP
llegan con HTTP 500 y también se leen.

```rust,no_run
use sunat_transport::{
    parse_send_bill_response, read_cdr, send_bill_envelope, unzip_xml, zip_document, zip_file_name,
    SendBillOutcome, TransportError, SOAP_ACTION_SEND_BILL,
};

fn send(
    name: &str,           // 20123456789-01-F001-1
    signed_xml: &[u8],
    sol_user: &str,       // RUC + usuario SOL
    sol_password: &str,
    post: impl Fn(&str, &str) -> Vec<u8>,
) -> Result<(), TransportError> {
    let zip = zip_document(name, signed_xml)?;
    let envelope = send_bill_envelope(sol_user, sol_password, &zip_file_name(name), &zip);
    match parse_send_bill_response(&post(SOAP_ACTION_SEND_BILL, &envelope))? {
        SendBillOutcome::Cdr(cdr_zip) => {
            let cdr = read_cdr(&unzip_xml(&cdr_zip)?)?;
            println!("{} {}", cdr.response_code, cdr.description); // "0": aceptado
        }
        SendBillOutcome::Fault { code, message } => println!("{code} {message}"),
    }
    Ok(())
}
```

## Los crates del SDK

| Crate | Qué hace |
|---|---|
| [`sunat-model`](https://crates.io/crates/sunat-model) | lee el JSON del documento y comprueba que esté dentro del alcance del SDK |
| [`sunat-ubl`](https://crates.io/crates/sunat-ubl) | arma el XML UBL en forma canónica |
| [`sunat-xmldsig`](https://crates.io/crates/sunat-xmldsig) | lo firma con tu certificado y verifica firmas |
| [`sunat-transport`](https://crates.io/crates/sunat-transport) | el ZIP, los sobres SOAP y la lectura de respuestas y CDR |
| [`sunat-sdk`](https://crates.io/crates/sunat-sdk) | cada llamada a la SUNAT como JSON: arma la petición y lee la respuesta |

Ninguno abre conexiones ni lee archivos: el POST HTTP lo haces tú, con TLS verificado. Se publican juntos y cada uno
pide a los otros en su versión exacta, así que agrégalos todos con la misma versión.

Cada versión pasa, compilada desde los paquetes que se suben a crates.io, la batería de vectores de compatibilidad del
[repositorio](https://github.com/Kembec/facturacion-electronica-sunat-sdk): el mismo `DigestValue` y el mismo
`SignatureValue` que dan JavaScript, Python, PHP y la línea de comandos para los mismos datos.

Licencia MIT ([LICENSE](LICENSE)). Este proyecto no está afiliado a la SUNAT ([NOTICE](NOTICE)).
