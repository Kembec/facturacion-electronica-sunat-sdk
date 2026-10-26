# SDK de facturación electrónica SUNAT (Perú) para Ruby

Un SDK para emitir comprobantes electrónicos ante la SUNAT desde **Ruby**, para quien programa el sistema de
facturación de una empresa. Con los datos de una factura (01), una boleta (03), una nota de crédito (07) o de débito
(08), un resumen diario de boletas (RC) o una comunicación de baja (RA):

- arma el archivo XML que pide la SUNAT y lo firma con tu certificado digital;
- lo comprime, lo envía y lee la CDR, la constancia con la que la SUNAT dice si lo aceptó;
- después consulta un comprobante ya enviado: su CDR (`status`), el ticket de un resumen (`ticket`) o su estado en la
  Consulta Integrada de la SUNAT (`validity`: aceptado, anulado o no informado).

También arma, firma y envía la guía de remisión remitente (09) por la API REST de la SUNAT. Es la versión para Ruby del
[SDK](https://github.com/Kembec/facturacion-electronica-sunat-sdk), que también funciona en .NET, Java, JavaScript y
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
[workflow `ruby`](https://github.com/Kembec/facturacion-electronica-sunat-sdk/actions/workflows/ruby.yml),
descomprímelo e instala la gema de tu plataforma (`ruby -e 'puts Gem::Platform.local'` te dice cuál es):

```bash
gem install ./kembec-sunat_sdk-1.0.0.rc.6-x86_64-linux-gnu.gem
```

| Plataforma | Gema |
|---|---|
| Linux x64 / ARM64, glibc 2.17 o posterior (no Alpine) | `…-x86_64-linux-gnu.gem` / `…-aarch64-linux-gnu.gem` |
| macOS Intel / Apple Silicon | `…-x86_64-darwin.gem` / `…-arm64-darwin.gem` |
| Windows de 64 bits (RubyInstaller con UCRT) | `…-x64-mingw-ucrt.gem` |
| Windows de 32 bits (RubyInstaller x86, que llega hasta Ruby 3.4) | `…-x86-mingw32.gem` |

Con Bundler, instálala primero así y fija la versión en tu `Gemfile`; `bundle install` toma la gema instalada. Cada
máquina donde corre `bundle install` (tu CI, tu servidor) necesita la gema instalada antes:

```ruby
gem "kembec-sunat_sdk", "1.0.0.rc.6"
```

Es una versión beta. Corre en Ruby 3.2 o posterior (probado en 3.2, 3.3, 3.4 y 4.0) y cada gema trae dentro el núcleo
en Rust del SDK ya compilado para su plataforma: no llama a ningún programa externo ni compila nada al instalarse. Su
única dependencia es [ffi](https://github.com/ffi/ffi). En otra plataforma, `KEMBEC_SUNAT_SDK_LIBRARY` apunta a una
biblioteca compilada desde
[`bindings/ffi`](https://github.com/Kembec/facturacion-electronica-sunat-sdk/tree/main/bindings/ffi).

## Tu primer comprobante

Baja a la carpeta de tu proyecto la factura de prueba y el certificado de prueba del repositorio. Ese certificado es
autogenerado y no sirve para emitir.

```bash
base=https://raw.githubusercontent.com/Kembec/facturacion-electronica-sunat-sdk/v1.0.0-rc.9
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
SDK funciona: es la misma salida que dan .NET, Java, JavaScript, PHP, Python, Rust y la CLI. `build` recibe el texto
JSON o un `Hash` con la forma del
[modelo de entrada](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/input-model.md), con los
importes como texto (`"148.00"`).

## Enviar a la SUNAT y leer la respuesta

```ruby
sdk = Kembec::SunatSdk::Client.new(http_timeout: 60)   # espera a la SUNAT: de 1 a 300 s enteros; 90 s si no se indica
built = sdk.build(document)                             # el JSON o el Hash de docs/input-model.md
signed = sdk.sign(built.xml, Kembec::SunatSdk::SigningKey.new("/seguro/cert.pem", "/seguro/key.pem"))
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
- `accepted?` es verdadero solo con código 0 y sin notas. Con código 0 y cualquier nota, sea cual sea su código, la CDR
  queda `:observed` y `valid_document?` es verdadero en los dos casos. El 2404 (una nota que modifica un comprobante
  físico) es observación aunque esté por debajo de 4000.
- `summary` envía un resumen (RC) o una baja (RA) y devuelve su ticket: guárdalo antes que nada. `ticket` y `status`
  sin respuesta dan `:no_verdict`: se vuelve a consultar, nunca a enviar.
- `send_bill`, `summary`, `ticket` y `status` traen `diagnosis`: por cada código de la respuesta (el del `:fault`, el de
  la CDR si no es 0 y cada nota que empieza con un código), el campo de tu JSON de entrada que pudo causarlo. Con el
  2800 de la beta: `sent.diagnosis.first.fields.first.input_path` es `"customer.document_type"` y
  `sent.diagnosis.first.detail` da `node` `"cbc:ID/schemeID"` y `value` `"1"`. `fields` está vacío para un código que
  la tabla no lista y para los resúmenes y las bajas; sin diagnóstico, `diagnosis` es una lista vacía.
- Cada resultado guarda en `raw_json` la respuesta tal como la da el
  [contrato de la CLI](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/cli.md).

### Consultar el estado de un comprobante (Consulta Integrada)

`validity` pregunta a la Consulta Integrada de la SUNAT si una factura, boleta o nota (01, 03, 07, 08) existe y en qué
estado está. Usa las «Credenciales de API SUNAT» que se crean en SOL para esta consulta: no el usuario SOL ni el par de
la GRE. **No hay ambiente de pruebas**: la consulta va siempre a producción.

```ruby
check = sdk.validity(ruc: "20123456789", document_type: "01", series: "F001", number: 123,
                     issue_date: Date.new(2026, 10, 16), total: "118.00",
                     credentials: Kembec::SunatSdk::ApiCredentials.new(client_id, client_secret))

case check.outcome
when :validity    then check.state               # :accepted, :voided, :not_reported…; check.observations dice qué difiere
when :query_error then check.query_error.message # la SUNAT rechazó la consulta, no el comprobante
else                   check.no_verdict.stage    # sin respuesta en :token o :query: vuelve a consultar
end
```

`issue_date` acepta `Date`, `Time` o `"AAAA-MM-DD"`; `total`, texto, `Integer` o `BigDecimal` (desde Ruby 3.4,
`bigdecimal` va en tu `Gemfile`). Un `Float` se rechaza con `ArgumentError`, porque no es un importe exacto.

Ningún resultado es motivo para emitir la operación con otro número: `:not_reported` no prueba que el comprobante no
llegó (el ticket de un resumen puede seguir en proceso), así que antes se concilia con `status` o con el ticket.

### Guía de remisión remitente (09) por la API REST

`gre_send` envía la guía firmada y devuelve su ticket; `gre_ticket` consulta el resultado con ese ticket. Usan el par
GRE de las «Credenciales de API SUNAT» y el usuario SOL.

```ruby
api = Kembec::SunatSdk::ApiCredentials.new(gre_client_id, gre_client_secret) # el par de GRE de «Credenciales de API SUNAT»
sol = Kembec::SunatSdk::SolCredentials.new(ruc_y_usuario_sol, clave_sol)
sent = sdk.gre_send(built.file_name, signed.xml, api: api, sol: sol, token_endpoint: token_endpoint, endpoint: gre_endpoint)

case sent.outcome
when :ticket
  ticket = sent.ticket # guárdalo antes que nada: es lo único que lleva al resultado
  status = sdk.gre_ticket(ticket, api: api, sol: sol, token_endpoint: token_endpoint, endpoint: gre_endpoint)
  # status.state: :in_process (98) se vuelve a consultar con el mismo ticket; :processed (0) trae status.cdr;
  # :processed_with_errors (99) trae status.error, con su código, descripción y response_class
when :api_error
  sent.api_error.errors # lo que la SUNAT rechazó, en sus palabras (code y description)
end
```

- **La SUNAT no tiene ambiente de pruebas para la GRE**, y ninguna guía de este SDK figura como aceptada por la SUNAT.
- Las dos URL base no tienen valor por defecto y solo se aceptan `https://` (las del manual:
  `https://api-seguridad.sunat.gob.pe/v1/clientessol` para el token y `https://api-cpe.sunat.gob.pe/v1` para la API).
- Si el envío no recibe respuesta, `gre_send` lanza `Kembec::SunatSdk::Error`: la guía pudo haber llegado, así que se
  reenvía el mismo archivo, nunca la operación con otro número. Sin token no se envía nada y el resultado es
  `:no_verdict`.

## Errores

| Excepción | Cuándo |
|---|---|
| `ArgumentError` | el texto del documento no es un valor JSON, una credencial o una ruta del certificado está vacía, falta un argumento con nombre, la espera no es de 1 a 300 s enteros o el total es `Float` |
| `TypeError` | un argumento es `nil` o de otro tipo |
| `Kembec::SunatSdk::Error` | el comando no corrió: el SDK rechazó el documento (el mensaje dice qué campo y por qué), falta el certificado, el endpoint no es `https://`, no hubo respuesta a un envío o la respuesta no es la del contrato; `command` dice qué comando |
| `Kembec::SunatSdk::NativeLibraryError` | subclase de la anterior: la biblioteca nativa no se encontró, no se pudo cargar en este proceso o es de otra versión de la gema |

La biblioteca se carga en la primera llamada, no en el `require`. Las contraseñas y secretos no aparecen en ningún
resultado, excepción ni `inspect`, y los bytes de la petición que los lleva se borran después de cada llamada.

## Qué cubre y qué no

Lo mismo que el SDK en cualquier lenguaje: la lista completa está en el
[alcance del SDK](https://github.com/Kembec/facturacion-electronica-sunat-sdk#alcance) y cada campo, en el
[modelo de entrada](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/input-model.md). Lo que
el SDK no cubre lo rechaza con un error explícito en vez de armar un XML que la SUNAT rechazaría. No hay gema para
JRuby, Alpine (musl) ni Windows ARM64.

## Pruebas

Lo que aceptó la SUNAT y cómo se comprueba cada versión está en
[Cómo se comprueba](https://github.com/Kembec/facturacion-electronica-sunat-sdk#cómo-se-comprueba): la beta de la SUNAT
aceptó 36 de los 38 documentos de ejemplo; las 2 guías de remisión no tienen beta.

Para esta gema, el CI compila el núcleo para las seis plataformas, arma una gema por plataforma, la instala como lo
harías tú y corre la batería en Ruby 3.2, 3.3, 3.4 y 4.0 (en Windows de 32 bits, 3.3 y 3.4, y falla si el proceso no
es de 32 bits): cada uno de los 38 documentos de
[fixtures](https://github.com/Kembec/facturacion-electronica-sunat-sdk/tree/main/fixtures) tiene que dar, con `build` y
`sign`, el XML y la firma exactos de
[tests/vectors](https://github.com/Kembec/facturacion-electronica-sunat-sdk/tree/main/tests/vectors). Además, ocho
hilos firman a la vez, y una llamada que espera a la SUNAT tiene que dejar correr a los demás hilos. `status`,
`validity` y `gre_ticket` se prueban sin respuesta contra un servidor local mudo, y `read_cdr` contra una CDR grabada; no
envía nada a la SUNAT.

## Licencia

MIT, © 2026 Manuel Benancio: ver
[`LICENSE`](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/LICENSE).
