# SDK de facturación electrónica SUNAT (Perú) para Swift

Un SDK para emitir comprobantes electrónicos ante la SUNAT desde **Swift**, en macOS, iOS y Linux, para quien programa
el sistema de facturación de una empresa. Con los datos de una factura (01), una boleta (03), una nota de crédito (07) o
de débito (08), un resumen diario de boletas (RC) o una comunicación de baja (RA):

- arma el archivo XML que pide la SUNAT y lo firma con tu certificado digital;
- lo comprime, lo envía y lee la CDR, la constancia con la que la SUNAT dice si lo aceptó;
- después consulta un comprobante ya enviado: su CDR (`status`), el ticket de un resumen (`ticket`) o su estado en la
  Consulta Integrada de la SUNAT (`validity`: aceptado, anulado o no informado).

También arma, firma y envía la guía de remisión remitente (09) por la API REST de la SUNAT. Es la versión para Swift del
[SDK](https://github.com/Kembec/facturacion-electronica-sunat-sdk), que también funciona en .NET, Java, JavaScript y
TypeScript, Objective-C, Perl, PHP, Python, Ruby, Rust y la línea de comandos: los mismos datos dan el mismo comprobante
firmado en todos. Para Objective-C, el mismo paquete tiene su propio producto, con [su README](../objc/README.md).

SUNAT (Peru) electronic invoicing SDK for Swift.

Tu sistema calcula los importes; el SDK los valida y los pone en el XML, pero no calcula impuestos. Guías y ejemplos:
https://fe.kembec.com

## Instalación

El paquete todavía no se puede agregar por URL. El tag `v0.1.0-beta.5` ya tiene `Package.swift`, pero sin binario:
en ese commit `release` es `nil`, y el release no adjunta `SunatSDKFFI.xcframework.zip`. SwiftPM resuelve la versión
0.1.0-beta.5 y se detiene en `local binary target 'SunatSDKFFI' … does not contain a binary artifact`. Se podrá
agregar por URL cuando un release adjunte ese zip y el commit de su tag nombre su URL y su checksum en `release`;
entonces SwiftPM bajará el binario y comprobará el checksum.

Hasta entonces, se usa una copia local del repositorio con el binario que arma el CI. `Package.swift` lee el
`SunatSDKFFI.xcframework` de `bindings/swift/artifacts`, que no está en git (son 139 MB): baja el artefacto
`SunatSDKFFI.xcframework` de una ejecución del
[workflow `swift`](https://github.com/Kembec/facturacion-electronica-sunat-sdk/actions/workflows/swift.yml) y
descomprímelo ahí. Con `gh`:

```bash
git clone https://github.com/Kembec/facturacion-electronica-sunat-sdk.git
cd facturacion-electronica-sunat-sdk
run=$(gh run list -w swift -b "$(git branch --show-current)" -s success -L 1 --json databaseId -q '.[0].databaseId')
gh run download "$run" -n SunatSDKFFI.xcframework -D bindings/swift/artifacts/SunatSDKFFI.xcframework
```

Después, agrégalo a tu paquete por ruta (en Xcode, con «Add Local…» sobre esa carpeta):

```swift
dependencies: [.package(path: "../facturacion-electronica-sunat-sdk")],
targets: [
    .executableTarget(name: "PrimerComprobante", dependencies: [
        .product(name: "KembecSunatSDK", package: "facturacion-electronica-sunat-sdk"),
    ]),
]
```

Es una versión beta (1.0.0-rc.3, la de `SunatSDKClient.packageVersion`). Pide Swift 5.9 o posterior y corre en:

| Plataforma | Núcleo del SDK |
|---|---|
| macOS 10.15 o posterior, Apple Silicon e Intel | `SunatSDKFFI.xcframework`, bibliotecas estáticas |
| iOS 13 o posterior, dispositivo y simulador (arm64 y x86_64) | `SunatSDKFFI.xcframework`, bibliotecas estáticas |
| Linux x64 / ARM64, glibc 2.17 o posterior (no Alpine) | `libsunat_sdk_ffi.so` |

En Apple, el núcleo en Rust queda enlazado dentro de tu binario: no hay framework que embeber ni firmar, ni programa
externo al que llamar. El ejemplo de abajo, compilado en release para macOS arm64, pesa 5,3 MB.

En Linux no hay xcframework: el paquete enlaza `libsunat_sdk_ffi.so`. Baja del mismo run el artefacto
`shared-linux-x86_64` o `shared-linux-aarch64` (compilado en manylinux2014) y pásale su carpeta al enlazador al compilar
y al cargador al correr:

```bash
swift build -Xlinker -L/ruta/a/la/so
LD_LIBRARY_PATH=/ruta/a/la/so .build/debug/PrimerComprobante
```

## Tu primer comprobante

Baja a la carpeta de tu proyecto la factura de prueba y el certificado de prueba del repositorio. Ese certificado es
autogenerado y no sirve para emitir.

```bash
base=https://raw.githubusercontent.com/Kembec/facturacion-electronica-sunat-sdk/v1.0.0-rc.3
curl -fsSLO "$base/fixtures/01-invoice-taxed-cash.json"
curl -fsSLO "$base/tests/vectors/test-cert/test-cert.pem"
curl -fsSLO "$base/tests/vectors/test-cert/test-key.pem"
```

```swift
import Foundation
import KembecSunatSDK

let sdk = try SunatSDKClient()
let built = try sdk.build(documentJSON: String(contentsOfFile: "01-invoice-taxed-cash.json", encoding: .utf8))
let signed = try sdk.sign(xml: built.xml, key: SigningKey(certificatePath: "test-cert.pem", privateKeyPath: "test-key.pem"))
print(built.fileName, signed.digestValue) // 20123456789-01-F001-1 +te61a2ub5MlTFf8tKTPNGWEXKI=
```

Si imprime `20123456789-01-F001-1 +te61a2ub5MlTFf8tKTPNGWEXKI=` (el nombre del archivo y la huella de la firma), el
SDK funciona: es la misma salida que dan .NET, Java, JavaScript, Objective-C, Perl, PHP, Python, Ruby, Rust y la CLI.
Tus propios documentos siguen la forma del
[modelo de entrada](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/input-model.md).
`build(_:)` también recibe cualquier valor `Encodable` con esa forma: lo codifica con un `JSONEncoder` sin estrategia
de claves, así que el snake_case lo ponen tus `CodingKeys`, y los importes van como texto (`"148.00"`).

## Enviar a la SUNAT y leer la respuesta

```swift
let sdk = try SunatSDKClient(httpTimeout: 60)            // espera a la SUNAT: de 1 a 300 s enteros; 90 s si no se indica
let built = try sdk.build(documentJSON: documentJSON)    // el JSON de docs/input-model.md
let signed = try sdk.sign(xml: built.xml, key: SigningKey(certificatePath: "/seguro/cert.pem", privateKeyPath: "/seguro/key.pem"))
let sol = SOLCredentials(user: rucAndSOLUser, password: solPassword)
let sent = try await sdk.send(name: built.fileName, signedXML: signed.xml, credentials: sol, endpoint: sendBillEndpoint)

switch sent.outcome {
case let .fault(fault):
    print(fault.code, fault.responseClass)               // por ejemplo, "1033" y .alreadyRegistered
    for diagnosis in sent.diagnosis {                    // un elemento por código de la respuesta
        print(diagnosis.code,                            // "2800"
              diagnosis.detail?.node,                    // lo que nombra la SUNAT: "cbc:ID/schemeID" = "1"
              diagnosis.fields.first?.inputPath)         // el campo de tu JSON: "customer.document_type"
    }
case let .cdr(cdr, zip):
    try zip?.write(to: URL(fileURLWithPath: "\(built.fileName).cdr.zip")) // guarda la CDR como llegó
    print(cdr.isValidDocument)                           // aceptado, con o sin observaciones
}
```

- Cada comando de red tiene dos formas: la síncrona, que bloquea su hilo hasta la espera del cliente, y la `async`, que
  corre la llamada en una cola global de Dispatch y no ocupa un hilo del pool cooperativo. `build`, `sign`, `zip` y
  `readCDR` no van a la red y solo son síncronos. `SunatSDKClient` es un struct `Sendable` sin estado: se comparte
  entre tareas e hilos.
- Un error SOAP de la SUNAT es un resultado (`.fault`), no un error. Un envío sin respuesta lanza
  `SunatSDKError.commandFailed`, porque pudo haber llegado: se reenvía el mismo XML firmado con el mismo número, nunca
  otro.
- `isAccepted` es verdadero solo con código 0 y sin notas. Con código 0 y cualquier nota, sea cual sea su código, la CDR
  queda `.observed` e `isValidDocument` es verdadero en los dos casos. El 2404 (una nota que modifica un comprobante
  físico) es observación aunque esté por debajo de 4000.
- `summary` envía un resumen (RC) o una baja (RA) y devuelve su ticket: guárdalo antes que nada. `ticket` y `status`
  sin respuesta dan `.noVerdict`: se vuelve a consultar, nunca a enviar.
- `diagnosis` (en `SendResult`, `SummaryResult`, `TicketResult` y `StatusResult`) dice qué campo de tu entrada causó
  cada código: el del rechazo, o el de cada nota de una CDR observada. Cada `Diagnosis` trae el `code`, el `detail` (nodo
  y valor que nombra la SUNAT, si nombra uno) y los `fields` candidatos, cada uno con su `inputPath`, su `xpath` y su
  `certainty` (`.source` si el nodo es el de la regla y un solo campo lo escribe; `.inferred` si no). `inputPath` es
  `nil` cuando el modelo de entrada no tiene ese campo, y `fields` queda vacío si el código no está en las reglas de la
  SUNAT del 26.08.2026. Sin `diagnosis` en la respuesta, la lista es vacía.
- Cada resultado es `Codable` con las claves del
  [contrato de la CLI](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/cli.md), así que
  `JSONEncoder().encode(sent)` da sus mismos campos; `rawJSON` guarda la respuesta tal como la dio la biblioteca y no
  se codifica. Un valor de enumeración que esta versión no conoce se lee como `.unknown`, no como error.

### Consultar el estado de un comprobante (Consulta Integrada)

`validity` pregunta a la Consulta Integrada de la SUNAT si una factura, boleta o nota (01, 03, 07, 08) existe y en qué
estado está. Usa las «Credenciales de API SUNAT» que se crean en SOL para esta consulta: no el usuario SOL ni el par de
la GRE. **No hay ambiente de pruebas**: la consulta va siempre a producción.

```swift
let check = try await sdk.validity(ruc: "20123456789", documentType: "01", series: "F001", number: 123,
                                   issueDate: DateComponents(year: 2026, month: 10, day: 16),
                                   total: Decimal(string: "118.00")!,
                                   credentials: APICredentials(clientID: clientID, clientSecret: clientSecret))
switch check.outcome {
case let .validity(validity): print(validity.state)    // .accepted, .voided, .notReported…; validity.observations dice qué difiere
case let .queryError(error):  print(error.message)     // la SUNAT rechazó la consulta, no el comprobante
case let .noVerdict(none):    print(none.message)      // sin respuesta en el token o en la consulta (none.stage): vuelve a consultar
}
```

`issueDate` toma el año, el mes y el día, e ignora lo demás. Crea `total` con `Decimal(string:)`: un literal como
`118.10` pasa por `Double` y deja de ser un importe exacto.

Ningún resultado es motivo para emitir la operación con otro número: `.notReported` no prueba que el comprobante no
llegó (el ticket de un resumen puede seguir en proceso), así que antes se concilia con `status` o con el ticket.

### Guía de remisión remitente (09) por la API REST

`greSend` envía la guía firmada y devuelve su ticket; `greTicket` consulta el resultado con ese ticket. Usan el par GRE
de las «Credenciales de API SUNAT» y el usuario SOL.

```swift
let api = APICredentials(clientID: greClientID, clientSecret: greClientSecret) // el par de GRE de «Credenciales de API SUNAT»
let sol = SOLCredentials(user: rucAndSOLUser, password: solPassword)
let sent = try await sdk.greSend(name: built.fileName, signedXML: signed.xml, api: api, sol: sol,
                                 tokenEndpoint: tokenEndpoint, endpoint: greEndpoint)

switch sent.outcome {
case let .ticket(guide):
    let ticket = guide.ticket                            // guárdalo antes que nada: es lo único que lleva al resultado
    let result = try await sdk.greTicket(ticket, api: api, sol: sol, tokenEndpoint: tokenEndpoint, endpoint: greEndpoint)
    if case let .ticketStatus(status) = result.outcome {
        // .inProcess (98) se vuelve a consultar con el mismo ticket; .processed (0) trae status.cdr;
        // .processedWithErrors (99) trae status.error, con su código, descripción y responseClass
        print(status.state)
    }
case let .apiError(error):
    print(error.errors)                                  // lo que la SUNAT rechazó, en sus palabras
case let .noVerdict(none):
    print(none.message)                                  // sin token: no se envió nada
}
```

- **La SUNAT no tiene ambiente de pruebas para la GRE**, y ninguna guía de este SDK figura como aceptada por la SUNAT.
- Las dos URL base no tienen valor por defecto y solo se aceptan `https://` (las del manual:
  `https://api-seguridad.sunat.gob.pe/v1/clientessol` para el token y `https://api-cpe.sunat.gob.pe/v1` para la API).
- Si el envío no recibe respuesta, `greSend` lanza `SunatSDKError.commandFailed`: la guía pudo haber llegado, así que
  se reenvía el mismo archivo, nunca la operación con otro número.

## Errores

Todos son `SunatSDKError`:

| Caso | Cuándo |
|---|---|
| `.invalidArgument` | antes de cualquier llamada: el texto del documento no es un valor JSON, la espera del cliente no es de 1 a 300 segundos enteros o la fecha no tiene año, mes y día |
| `.commandFailed(command:message:)` | el comando corrió y falló: el SDK rechazó el documento (el mensaje dice qué campo y por qué), falta el certificado, el endpoint no es `https://` o no hubo respuesta a un envío |
| `.loadFailed` | la biblioteca nativa enlazada no es la de este paquete: otra ABI u otra versión |
| `.unexpectedResponse(command:message:)` | la biblioteca respondió algo que no es la forma del contrato para ese comando |

`error.command` dice qué comando falló. Las contraseñas y secretos no aparecen en ningún resultado ni error, y
`SOLCredentials` y `APICredentials` los ocultan en `description`, `debugDescription` y `dump`; los bytes de la petición
que los lleva se borran después de cada llamada.

## Qué cubre y qué no

Lo mismo que el SDK en cualquier lenguaje: la lista completa está en el
[alcance del SDK](https://github.com/Kembec/facturacion-electronica-sunat-sdk#alcance) y cada campo, en el
[modelo de entrada](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/input-model.md). Lo que
el SDK no cubre lo rechaza con un error explícito en vez de armar un XML que la SUNAT rechazaría. El xcframework no
trae Mac Catalyst, tvOS, watchOS ni visionOS, y no hay build para Windows ni Alpine (musl).

## Pruebas

Lo que aceptó la SUNAT y cómo se comprueba cada versión está en
[Cómo se comprueba](https://github.com/Kembec/facturacion-electronica-sunat-sdk#cómo-se-comprueba): la beta de la SUNAT
aceptó 36 de los 38 documentos de ejemplo; las 2 guías de remisión no tienen beta.

Para este paquete, el CI compila el núcleo como biblioteca estática para macOS (arm64 y x86_64), iOS y el simulador de
iOS (arm64 y x86_64), arma el xcframework y corre la batería enlazándolo como lo haría tu proyecto: en macOS arm64 y
x86_64, en el simulador de iOS (como XCTest) y en Linux x64 y ARM64 con Swift 5.9 y 6.3 sobre la `.so` de
manylinux2014. El slice de dispositivo iOS se enlaza sin firmar; no se ejecuta. Cada uno de los 38 documentos de
[fixtures](https://github.com/Kembec/facturacion-electronica-sunat-sdk/tree/main/fixtures) tiene que dar, con `build`
y `sign`, el XML y la firma exactos de
[tests/vectors](https://github.com/Kembec/facturacion-electronica-sunat-sdk/tree/main/tests/vectors). Además, varios
hilos firman a la vez, cada resultado se codifica y decodifica con `Codable`, y las formas `async` se prueban junto a
las síncronas. `status`, `validity` y `greTicket` se prueban sin respuesta contra un servidor local mudo, y `readCDR`
contra una CDR grabada; no envía nada a la SUNAT.

## Licencia

MIT, © 2026 Manuel Benancio: ver
[`LICENSE`](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/LICENSE).
