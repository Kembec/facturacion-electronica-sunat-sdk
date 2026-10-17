# SDK de facturación electrónica SUNAT (Perú) para Ruby

Un SDK para emitir comprobantes electrónicos ante la SUNAT desde **Ruby**, para quien programa el sistema de
facturación de una empresa. Con los datos de una factura (01), una boleta (03), una nota de crédito (07) o de débito
(08), un resumen diario de boletas (RC) o una comunicación de baja (RA):

- arma el archivo XML que pide la SUNAT y lo firma con tu certificado digital;
- lo comprime, lo envía y lee la CDR, la constancia con la que la SUNAT dice si lo aceptó;
- después consulta un comprobante ya enviado: su CDR (`status`), el ticket de un resumen (`ticket`) o su estado en la
  Consulta Integrada de la SUNAT (`validity`: aceptado, anulado o no informado).

También arma, firma y envía la guía de remisión remitente (09) por la API REST de la SUNAT. Es la versión para Ruby del
[SDK](https://github.com/Kembec/facturacion-electronica-sunat-sdk), que también funciona en .NET, JavaScript y
TypeScript, PHP, Python, Rust y la línea de comandos: los mismos datos dan el mismo comprobante firmado en todos.

SUNAT (Peru) electronic invoicing SDK for Ruby.

Tu sistema calcula los importes; el SDK los valida y los pone en el XML, pero no calcula impuestos. Guías y ejemplos:
https://fe.kembec.com

## Instalación

La gema todavía no está en RubyGems. Cuando se publique, se instalará con:

```bash
gem install kembec-sunat_sdk --pre
```

Hasta entonces, el CI del repositorio arma las gemas en cada cambio: baja el artefacto `gems` de una ejecución del
[workflow `ruby`](https://github.com/Kembec/facturacion-electronica-sunat-sdk/actions/workflows/ruby.yml) e instala la
de tu plataforma, por ejemplo `gem install ./kembec-sunat_sdk-0.1.0.beta.4-x86_64-linux-gnu.gem`.

Es una versión beta. Corre en Ruby 3.2 o posterior (probado en 3.2, 3.3, 3.4 y 4.0) y cada gema trae dentro el núcleo
en Rust del SDK ya compilado para su plataforma: no llama a ningún programa externo ni compila nada al instalarse. Su
única dependencia es [ffi](https://github.com/ffi/ffi). Hay gema para Linux x64 y ARM64 con glibc 2.17 o posterior (no
Alpine), macOS Intel y Apple Silicon, y Windows de 64 bits (RubyInstaller con UCRT) y de 32 bits (RubyInstaller x86,
que llega hasta Ruby 3.4). En otra plataforma, `KEMBEC_SUNAT_SDK_LIBRARY` apunta a una biblioteca compilada desde
`bindings/ffi`.

## Tu primer comprobante

Baja a la carpeta de tu proyecto la factura de prueba y el certificado de prueba del repositorio. Ese certificado es
autogenerado y no sirve para emitir.

```bash
base=https://raw.githubusercontent.com/Kembec/facturacion-electronica-sunat-sdk/v0.1.0-beta.4
curl -fsSLO "$base/fixtures/01-invoice-taxed-cash.json"
curl -fsSLO "$base/tests/vectors/test-cert/test-cert.pem"
curl -fsSLO "$base/tests/vectors/test-cert/test-key.pem"
```

```ruby
require "kembec/sunat_sdk"

sdk = Kembec::SunatSdk::Client.new
built = sdk.build(File.read("01-invoice-taxed-cash.json"))
signed = sdk.sign(built.xml, Kembec::SunatSdk::SigningKey.new("test-cert.pem", "test-key.pem"))
puts "#{built.file_name} #{signed.digest_value}" # 20123456789-01-F001-1 +te61a2ub5MlTFf8tKTPNGWEXKI=
```

Si imprime `20123456789-01-F001-1 +te61a2ub5MlTFf8tKTPNGWEXKI=` (el nombre del archivo y la huella de la firma), el
SDK funciona: es la misma salida que dan .NET, JavaScript, PHP, Python, Rust y la CLI. `build` recibe el texto JSON o un
`Hash` con la forma del
[modelo de entrada](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/input-model.md), con los
importes como texto (`"148.00"`).

## Enviar a la SUNAT y leer la respuesta

```ruby
sdk = Kembec::SunatSdk::Client.new(http_timeout: 60)   # espera a la SUNAT: de 1 a 300 s enteros; 90 s si no se indica
sol = Kembec::SunatSdk::SolCredentials.new(ruc_y_usuario_sol, clave_sol)
sent = sdk.send_bill(built.file_name, signed.xml, credentials: sol, endpoint: send_bill_endpoint)

case sent.outcome
when :fault then sent.fault.code                        # por ejemplo, "1033", con su response_class
when :cdr
  File.binwrite("#{built.file_name}.cdr.zip", sent.cdr_zip) # guarda la CDR como llegó
  sent.cdr.valid_document?                              # aceptado, con o sin observaciones
end
```

- El comando `send` del contrato se llama aquí `send_bill`, como la operación de la SUNAT, porque `send` ya es de
  todo objeto Ruby.
- Mientras espera a la SUNAT, la llamada suelta el GVL: los demás hilos de tu proceso (Puma, Sidekiq) siguen
  trabajando. El cliente no guarda estado y se puede compartir entre hilos.
- Un error SOAP de la SUNAT es un resultado (`:fault`), no una excepción. Un envío sin respuesta lanza
  `Kembec::SunatSdk::Error`, porque pudo haber llegado: se reenvía el mismo XML firmado con el mismo número, nunca otro.
- `accepted?` es verdadero solo con código 0 y sin observaciones; con código 0 y una nota de 4000 o más, la CDR queda
  `:observed` y `valid_document?` es verdadero en los dos casos.
- `summary` envía un resumen (RC) o una baja (RA) y devuelve su ticket: guárdalo antes que nada. `ticket` y `status`
  sin respuesta dan `:no_verdict`: se vuelve a consultar, nunca a enviar.
- Cada resultado guarda en `raw_json` la respuesta tal como la da el
  [contrato de la CLI](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/cli.md).

### Consulta Integrada y guía de remisión

`validity` usa las «Credenciales de API SUNAT» de la Consulta Integrada (no el usuario SOL) y va siempre a producción:

```ruby
check = sdk.validity(ruc: "20123456789", document_type: "01", series: "F001", number: 123,
                     issue_date: Date.new(2026, 10, 16), total: BigDecimal("118.00"),
                     credentials: Kembec::SunatSdk::ApiCredentials.new(client_id, client_secret))
check.state # :accepted, :voided, :not_reported…; con :query_error o :no_verdict, vuelve a consultar
```

`gre_send` y `gre_ticket` envían la guía (09) y consultan su ticket con el par GRE y el usuario SOL. La SUNAT no tiene
ambiente de pruebas para la GRE, las dos URL base no tienen valor por defecto y solo se aceptan `https://`. El total
no se acepta como `Float`: un importe va como `BigDecimal`, `Integer` o texto.

## Errores

| Excepción | Cuándo |
|---|---|
| `ArgumentError` | el texto del documento no es un valor JSON, un texto está vacío, la espera no es de 1 a 300 s enteros o el total es `Float` |
| `TypeError` | falta un argumento o es de otro tipo |
| `Kembec::SunatSdk::Error` | el comando no corrió: el SDK rechazó el documento (el mensaje dice qué campo y por qué), falta el certificado, el endpoint no es `https://`, no hubo respuesta a un envío; `command` dice cuál |
| `Kembec::SunatSdk::NativeLibraryError` | subclase de la anterior: la biblioteca nativa no se pudo cargar o es de otra versión |

Las contraseñas y secretos no aparecen en ningún resultado, excepción ni `inspect`.

## Pruebas

Para esta gema, el CI compila el núcleo para las seis plataformas, arma una gema por plataforma, la instala como lo
harías tú y corre la batería en Ruby 3.2, 3.3, 3.4 y 4.0 (en Windows de 32 bits, 3.3 y 3.4): cada uno de los 38
documentos de [fixtures](https://github.com/Kembec/facturacion-electronica-sunat-sdk/tree/main/fixtures) tiene que
dar, con `build` y `sign`, el XML y la firma exactos de
[tests/vectors](https://github.com/Kembec/facturacion-electronica-sunat-sdk/tree/main/tests/vectors). `status`,
`validity` y `gre_ticket` se prueban sin respuesta contra un servidor local mudo, y `read_cdr` contra CDR grabadas; no
envía nada a la SUNAT.

## Licencia

MIT, © 2026 Manuel Benancio: ver
[`LICENSE`](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/LICENSE).
