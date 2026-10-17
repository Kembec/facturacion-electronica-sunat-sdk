# SDK de facturación electrónica SUNAT (Perú) para Perl

Un SDK para emitir comprobantes electrónicos ante la SUNAT desde **Perl**, para quien programa el sistema de
facturación de una empresa. Con los datos de una factura (01), una boleta (03), una nota de crédito (07) o de débito
(08), un resumen diario de boletas (RC) o una comunicación de baja (RA):

- arma el archivo XML que pide la SUNAT y lo firma con tu certificado digital;
- lo comprime, lo envía y lee la CDR, la constancia con la que la SUNAT dice si lo aceptó;
- después consulta un comprobante ya enviado: su CDR (`status`), el ticket de un resumen (`ticket`) o su estado en la
  Consulta Integrada de la SUNAT (`validity`: aceptado, anulado o no informado).

También arma, firma y envía la guía de remisión remitente (09) por la API REST de la SUNAT. Es la versión para Perl del
[SDK](https://github.com/Kembec/facturacion-electronica-sunat-sdk), que también funciona en .NET, Java, JavaScript y
TypeScript, PHP, Python, Ruby, Rust y la línea de comandos: los mismos datos dan el mismo comprobante firmado en todos.

SUNAT (Peru) electronic invoicing SDK for Perl.

Tu sistema calcula los importes; el SDK los valida y los pone en el XML, pero no calcula impuestos. Guías y ejemplos:
https://fe.kembec.com

## Instalación

La distribución `Kembec-SunatSdk` todavía no está en CPAN. Cuando se publique, se instalará con:

```bash
cpanm --dev Kembec::SunatSdk
```

Hasta entonces, el CI del repositorio arma la distribución en cada cambio: baja el artefacto `cpan-dist` de una
ejecución del
[workflow `perl`](https://github.com/Kembec/facturacion-electronica-sunat-sdk/actions/workflows/perl.yml),
descomprímelo e instala el `.tar.gz` (con `cpanm`, o con `cpan` y la misma ruta):

```bash
cpanm ./Kembec-SunatSdk-0.000999_004.tar.gz
```

La misma distribución sirve en todas las plataformas: trae el núcleo en Rust del SDK ya compilado para cada una, y al
instalarse copia solo el de tu máquina. No llama a ningún programa externo ni compila Rust.

| Plataforma | Perl |
|---|---|
| Linux x64 / ARM64, glibc 2.17 o posterior (no Alpine) | el del sistema o el tuyo, 5.26 o posterior |
| macOS Intel / Apple Silicon | el del sistema o el tuyo, 5.26 o posterior |
| Windows de 64 bits | Strawberry Perl de 64 bits |

`0.000999_004` es la 0.1.0-beta.4 del SDK en la numeración de CPAN, una versión *trial*; 0.1.0 será `0.001000`. Corre
en Perl 5.26 o posterior (probado en 5.40, 5.42 y 5.44 en Linux y macOS, en 5.26 en Linux x64 y en Strawberry Perl 5.40
y 5.42 en Windows x64). Su única dependencia que no viene con Perl es
[FFI::Platypus](https://metacpan.org/pod/FFI::Platypus), que `cpanm` instala y compila con el compilador de C de tu
sistema (Strawberry Perl ya lo trae).
En otra plataforma, `KEMBEC_SUNAT_SDK_LIBRARY` apunta a una biblioteca compilada desde
[`bindings/ffi`](https://github.com/Kembec/facturacion-electronica-sunat-sdk/tree/main/bindings/ffi); sin ella, la
instalación se detiene con «OS unsupported».

## Tu primer comprobante

Baja a la carpeta de tu proyecto la factura de prueba y el certificado de prueba del repositorio. Ese certificado es
autogenerado y no sirve para emitir.

```bash
base=https://raw.githubusercontent.com/Kembec/facturacion-electronica-sunat-sdk/v0.1.0-beta.4
curl -fsSLO "$base/fixtures/01-invoice-taxed-cash.json"
curl -fsSLO "$base/tests/vectors/test-cert/test-cert.pem"
curl -fsSLO "$base/tests/vectors/test-cert/test-key.pem"
```

```perl
use Kembec::SunatSdk;

my $sdk = Kembec::SunatSdk->new;
my $document = do { local $/; open my $fh, '<:raw', '01-invoice-taxed-cash.json' or die $!; <$fh> };
my $built = $sdk->build($document);
my $signed = $sdk->sign($built->xml, Kembec::SunatSdk::SigningKey->new('test-cert.pem', 'test-key.pem'));
print $built->file_name, ' ', $signed->digest_value, "\n"; # 20123456789-01-F001-1 +te61a2ub5MlTFf8tKTPNGWEXKI=
```

Si imprime `20123456789-01-F001-1 +te61a2ub5MlTFf8tKTPNGWEXKI=` (el nombre del archivo y la huella de la firma), el
SDK funciona: es la misma salida que dan .NET, Java, JavaScript, PHP, Python, Ruby, Rust y la CLI. `build` recibe el
texto JSON o una referencia a un hash con la forma del
[modelo de entrada](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/input-model.md), con los
importes como texto (`"148.00"`).

## Enviar a la SUNAT y leer la respuesta

```perl
my $sdk = Kembec::SunatSdk->new(http_timeout => 60); # espera a la SUNAT: de 1 a 300 s enteros; 90 s si no se indica
my $built = $sdk->build($document);                   # el JSON o el hash de docs/input-model.md
my $signed = $sdk->sign($built->xml, Kembec::SunatSdk::SigningKey->new('/seguro/cert.pem', '/seguro/key.pem'));
my $sol = Kembec::SunatSdk::SolCredentials->new($ruc_y_usuario_sol, $clave_sol);
my $sent = $sdk->send($built->file_name, $signed->xml, credentials => $sol, endpoint => $send_bill_endpoint);

if ($sent->outcome eq 'fault') {
    print $sent->fault->code, "\n";                   # por ejemplo, "1033", con su response_class
}
else {                                                # 'cdr'
    open my $fh, '>:raw', $built->file_name . '.cdr.zip' or die $!;
    print {$fh} $sent->cdr_zip;                       # guarda la CDR como llegó
    close $fh;
    $sent->cdr->is_valid_document;                    # aceptado, con o sin observaciones
}
```

- Cada método lleva el nombre del comando del contrato: `send`, `summary`, `ticket`, `status`, `validity`, `gre_send`,
  `gre_ticket`, `zip` y `read_cdr`. Los argumentos con nombre se escriben `nombre => valor`; uno que falta o que no
  existe se rechaza en vez de pasar en silencio.
- `outcome` es el texto del contrato (`'cdr'`, `'fault'`, `'no_verdict'`…), y también `state` y `response_class`.
- Un error SOAP de la SUNAT es un resultado (`'fault'`), no una excepción. Un envío sin respuesta lanza
  `Kembec::SunatSdk::Error`, porque pudo haber llegado: se reenvía el mismo XML firmado con el mismo número, nunca otro.
- `is_accepted` es verdadero solo con código 0 y sin observaciones; con código 0 y una nota de 4000 o más, la CDR
  queda `'observed'` y `is_valid_document` es verdadero en los dos casos.
- `summary` envía un resumen (RC) o una baja (RA) y devuelve su ticket: guárdalo antes que nada. `ticket` y `status`
  sin respuesta dan `'no_verdict'`: se vuelve a consultar, nunca a enviar.
- Cada resultado guarda en `raw_json` la respuesta tal como la da el
  [contrato de la CLI](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/cli.md).

### Texto y bytes

El SDK recibe el texto como caracteres de Perl o como bytes en UTF-8 (lo que da un archivo abierto con `:raw`); un
texto en otra codificación, como Latin-1, se rechaza con `ArgumentError` en vez de leerse mal. El XML que devuelve son
caracteres: se guarda con `:encoding(UTF-8)`. El zip (`content` de `zip`) y la CDR (`cdr_zip`) son bytes: se guardan
con `:raw`, y `read_cdr` recibe bytes.

### Procesos e hilos

El objeto no guarda estado entre llamadas. Se carga antes del `fork` de un servidor *prefork* (Starman, Apache con
mod_perl) y cada hijo lo sigue usando, y se comparte entre hilos (`threads`). La biblioteca nativa se carga en la
primera llamada y queda cargada hasta que termina el proceso: ni un objeto destruido ni un hilo que termina la
descargan.

### Consultar el estado de un comprobante (Consulta Integrada)

`validity` pregunta a la Consulta Integrada de la SUNAT si una factura, boleta o nota (01, 03, 07, 08) existe y en qué
estado está. Usa las «Credenciales de API SUNAT» que se crean en SOL para esta consulta: no el usuario SOL ni el par de
la GRE. **No hay ambiente de pruebas**: la consulta va siempre a producción.

```perl
my $check = $sdk->validity(ruc => '20123456789', document_type => '01', series => 'F001', number => 123,
    issue_date => '2026-10-16', total => '118.00',
    credentials => Kembec::SunatSdk::ApiCredentials->new($client_id, $client_secret));

if    ($check->outcome eq 'validity')    { $check->state }               # 'accepted', 'voided', 'not_reported'…; observations dice qué difiere
elsif ($check->outcome eq 'query_error') { $check->query_error->message } # la SUNAT rechazó la consulta, no el comprobante
else                                     { $check->no_verdict->stage }    # sin respuesta en 'token' o 'query': vuelve a consultar
```

`issue_date` acepta `"AAAA-MM-DD"`, un `DateTime` o un `Time::Piece`; `total`, texto, un entero o un `Math::BigFloat`.
Un número de coma flotante (`118.0`) se rechaza con `ArgumentError`, porque no es un importe exacto.

Ningún resultado es motivo para emitir la operación con otro número: `'not_reported'` no prueba que el comprobante no
llegó (el ticket de un resumen puede seguir en proceso), así que antes se concilia con `status` o con el ticket.

### Guía de remisión remitente (09) por la API REST

`gre_send` envía la guía firmada y devuelve su ticket; `gre_ticket` consulta el resultado con ese ticket. Usan el par
GRE de las «Credenciales de API SUNAT» y el usuario SOL.

```perl
my $api = Kembec::SunatSdk::ApiCredentials->new($gre_client_id, $gre_client_secret); # el par de GRE
my $sol = Kembec::SunatSdk::SolCredentials->new($ruc_y_usuario_sol, $clave_sol);
my %gre = (api => $api, sol => $sol, token_endpoint => $token_endpoint, endpoint => $gre_endpoint);
my $sent = $sdk->gre_send($built->file_name, $signed->xml, %gre);

if ($sent->outcome eq 'ticket') {
    my $ticket = $sent->ticket; # guárdalo antes que nada: es lo único que lleva al resultado
    my $status = $sdk->gre_ticket($ticket, %gre);
    # $status->state: 'in_process' (98) se vuelve a consultar con el mismo ticket; 'processed' (0) trae
    # $status->cdr; 'processed_with_errors' (99) trae $status->error, con su code, description y response_class
}
elsif ($sent->outcome eq 'api_error') {
    $sent->api_error->errors; # lo que la SUNAT rechazó, en sus palabras (code y description)
}
```

- **La SUNAT no tiene ambiente de pruebas para la GRE**, y ninguna guía de este SDK figura como aceptada por la SUNAT.
- Las dos URL base no tienen valor por defecto y solo se aceptan `https://` (las del manual:
  `https://api-seguridad.sunat.gob.pe/v1/clientessol` para el token y `https://api-cpe.sunat.gob.pe/v1` para la API).
- Si el envío no recibe respuesta, `gre_send` lanza `Kembec::SunatSdk::Error`: la guía pudo haber llegado, así que se
  reenvía el mismo archivo, nunca la operación con otro número. Sin token no se envía nada y el resultado es
  `'no_verdict'`.

## Errores

| Excepción | Cuándo |
|---|---|
| `Kembec::SunatSdk::ArgumentError` | falta un argumento o no existe, es `undef`, de otro tipo o está vacío, el texto no es UTF-8 o no es un valor JSON, la espera no es de 1 a 300 s enteros o el total es un número de coma flotante; el mensaje termina, como `croak`, con la línea de tu código que hizo la llamada |
| `Kembec::SunatSdk::Error` | el comando no corrió: el SDK rechazó el documento (`message` dice qué campo y por qué), falta el certificado, el endpoint no es `https://`, no hubo respuesta a un envío o la respuesta no es la del contrato; `command` dice qué comando |
| `Kembec::SunatSdk::NativeLibraryError` | subclase de la anterior: la biblioteca nativa no se encontró, no se pudo cargar en este proceso o es de otra versión de la distribución |

```perl
my $built = eval { $sdk->build($document) };
if (my $error = $@) {
    die $error unless ref $error && $error->isa('Kembec::SunatSdk::Error');
    warn $error->command, ': ', $error->message, "\n";
}
```

La biblioteca se carga en la primera llamada, no en el `use`. Las contraseñas y secretos no aparecen en ningún
resultado, excepción ni al imprimir las credenciales (tampoco con `Data::Dumper`), y los bytes de la petición que los
lleva se borran después de cada llamada.

## Qué cubre y qué no

Lo mismo que el SDK en cualquier lenguaje: la lista completa está en el
[alcance del SDK](https://github.com/Kembec/facturacion-electronica-sunat-sdk#alcance) y cada campo, en el
[modelo de entrada](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/input-model.md). Lo que
el SDK no cubre lo rechaza con un error explícito en vez de armar un XML que la SUNAT rechazaría. No hay biblioteca
para Alpine (musl), Windows ARM64 ni Windows de 32 bits: Strawberry Perl no publica 32 bits después de la 5.38.2.

## Pruebas

Lo que aceptó la SUNAT y cómo se comprueba cada versión está en
[Cómo se comprueba](https://github.com/Kembec/facturacion-electronica-sunat-sdk#cómo-se-comprueba): la beta de la SUNAT
aceptó 36 de los 38 documentos de ejemplo; las 2 guías de remisión no tienen beta.

Para esta distribución, el CI compila el núcleo para las cinco plataformas, arma la distribución, la instala con
`cpanm` como lo harías tú (eso corre también sus pruebas de `t/`) y corre la batería en Perl 5.40, 5.42 y 5.44 en Linux
y macOS, en 5.26, la mínima, en Linux x64, y en Strawberry Perl 5.40 y 5.42 en Windows x64: cada uno de los 38
documentos de [fixtures](https://github.com/Kembec/facturacion-electronica-sunat-sdk/tree/main/fixtures) tiene que
dar, con `build` y `sign`, el XML y la firma exactos de
[tests/vectors](https://github.com/Kembec/facturacion-electronica-sunat-sdk/tree/main/tests/vectors), y falla si el
módulo no es el instalado o la biblioteca no es la de su plataforma. Además, ocho procesos hijos y ocho hilos firman a
la vez (en Windows, donde `fork` lo emula Perl con hilos, solo los hilos). `status`, `validity` y `gre_ticket` se prueban sin respuesta contra un servidor local mudo, y `read_cdr` contra
una CDR grabada; no envía nada a la SUNAT.

## Licencia

MIT, © 2026 Manuel Benancio: ver
[`LICENSE`](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/LICENSE).
