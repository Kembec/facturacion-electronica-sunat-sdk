# sunat-xmldsig

Firma XML-DSig envuelta para los documentos de la SUNAT, en el perfil de su manual del programador y de las guías UBL
2.1: RSA-SHA1, huella SHA-1 y C14N inclusiva 1.0, con la firma dentro de `ext:ExtensionContent`. También verifica una
firma. Es parte del [SDK de facturación electrónica SUNAT](https://github.com/Kembec/facturacion-electronica-sunat-sdk)
para Rust.

Enveloped XML-DSig for SUNAT (Peru) electronic documents.

La firma es determinista: para el mismo documento y la misma llave, `DigestValue` y `SignatureValue` son idénticos
byte a byte a los de los vectores de compatibilidad. La llave privada nunca aparece en un error, un log ni en
`Debug`. Está hecho para firmar el XML que arma `sunat-ubl`, no XML cualquiera.

## Instalación

```bash
cargo add sunat-xmldsig@0.1.0-beta.4
```

## Firmar y verificar

El certificado va en PEM (la cadena completa, si la tienes) y la llave en PEM sin cifrar (PKCS#8 o PKCS#1).

```rust,no_run
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let unsigned = fs::read("20123456789-01-F001-1.xml")?; // el XML de sunat_ubl::build
    let signed = sunat_xmldsig::sign(
        &unsigned,
        &fs::read_to_string("certificado.pem")?,
        &fs::read_to_string("llave.pem")?,
    )?;
    let check = sunat_xmldsig::verify(&signed)?;
    assert!(check.digest_matches && check.signature_matches);
    Ok(())
}
```

El recorrido completo, del JSON al XML firmado, está en el README de [`sunat-ubl`](https://crates.io/crates/sunat-ubl).

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
