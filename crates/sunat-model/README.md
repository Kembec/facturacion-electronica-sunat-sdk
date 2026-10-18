# kembec-sunat-model

El modelo de entrada del [SDK de facturación electrónica SUNAT](https://github.com/Kembec/facturacion-electronica-sunat-sdk)
para Rust: lee el JSON de una factura (01), una boleta (03), una nota de crédito (07) o de débito (08), un resumen
diario (RC), una comunicación de baja (RA) o una guía de remisión remitente (09), y rechaza lo que el SDK no sabe
emitir antes de que llegue a armarse un XML. Es para quien programa en Rust el sistema de facturación de una empresa.

Input model of the SUNAT (Peru) e-invoicing SDK.

Tu sistema calcula los importes y los manda como texto (`"118.00"`); este crate los valida, no calcula impuestos. La
forma del JSON está en el [modelo de entrada](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/input-model.md). Guías y ejemplos: https://fe.kembec.com

## Instalación

Requiere Rust 1.88 o posterior.

```bash
cargo add kembec-sunat-model
```

## Uso

Baja a una carpeta vacía la factura de ejemplo y el certificado de prueba del repositorio. Ese certificado es
autogenerado y no sirve para emitir.

```bash
base=https://raw.githubusercontent.com/Kembec/facturacion-electronica-sunat-sdk/v1.0.0-rc.1
curl -fsSLO "$base/fixtures/01-invoice-taxed-cash.json"
curl -fsSLO "$base/tests/vectors/test-cert/test-cert.pem"
curl -fsSLO "$base/tests/vectors/test-cert/test-key.pem"
```

```rust,no_run
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let document = kembec_sunat_model::Document::from_json(&fs::read_to_string("01-invoice-taxed-cash.json")?)?;
    println!("{}", document.file_name()); // 20123456789-01-F001-1
    Ok(())
}
```

Lo que el SDK no sabe escribir, o lo que la SUNAT rechazaría, vuelve como `ModelError` con la ruta del campo
(`$.lines[2].affectation`) y, si es una regla de la SUNAT, el código con el que contestaría (`1001`). Lo que sigue, armar el XML y firmarlo,
está en el README de [`kembec-sunat-ubl`](https://crates.io/crates/kembec-sunat-ubl).

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
`SignatureValue` que dan las versiones del SDK en los otros lenguajes y la línea de comandos para los mismos datos.

## Licencia

MIT, © 2026 Manuel Benancio: ver
[`LICENSE`](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/LICENSE). Este proyecto no está afiliado a la
SUNAT ([`NOTICE`](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/NOTICE)).
