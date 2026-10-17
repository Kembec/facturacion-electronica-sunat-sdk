# sunat-ubl

Arma el XML que pide la SUNAT a partir de un documento de `sunat-model`: UBL 2.1 para facturas, boletas, notas de
crédito y de débito y guías de remisión remitente, y UBL 2.0 para resúmenes diarios y comunicaciones de baja. Es parte
del [SDK de facturación electrónica SUNAT](https://github.com/Kembec/facturacion-electronica-sunat-sdk) para Rust.

UBL 2.1 and 2.0 XML for SUNAT (Peru) electronic invoicing.

El XML sale en forma canónica (C14N inclusiva 1.0), así que lo que se firma es exactamente lo que se armó. Los importes
se escriben como llegan: este crate no calcula impuestos ni totales.

## Instalación

```bash
cargo add sunat-model@0.1.0-beta.4 sunat-ubl@0.1.0-beta.4 sunat-xmldsig@0.1.0-beta.4
```

## Tu primer comprobante firmado

Baja a una carpeta vacía la factura de ejemplo y el certificado de prueba del repositorio. Ese certificado es
autogenerado y no sirve para emitir.

```bash
base=https://raw.githubusercontent.com/Kembec/facturacion-electronica-sunat-sdk/v0.1.0-beta.4
curl -fsSLO "$base/fixtures/01-invoice-taxed-cash.json"
curl -fsSLO "$base/tests/vectors/test-cert/test-cert.pem"
curl -fsSLO "$base/tests/vectors/test-cert/test-key.pem"
```

```rust,no_run
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

El archivo firmado lleva `<ds:DigestValue>+te61a2ub5MlTFf8tKTPNGWEXKI=</ds:DigestValue>`: la misma huella que da el SDK
en cualquier lenguaje para esa factura. Tus propios documentos siguen la forma del
[modelo de entrada](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/input-model.md).

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
