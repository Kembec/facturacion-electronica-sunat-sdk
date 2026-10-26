# SDK de facturación electrónica SUNAT (Perú) para Objective-C

Un SDK para emitir comprobantes electrónicos ante la SUNAT desde **Objective-C**, en macOS e iOS, para quien programa el
sistema de facturación de una empresa. Con los datos de una factura (01), una boleta (03), una nota de crédito (07) o
de débito (08), un resumen diario de boletas (RC) o una comunicación de baja (RA):

- arma el archivo XML que pide la SUNAT y lo firma con tu certificado digital;
- lo comprime, lo envía y lee la CDR, la constancia con la que la SUNAT dice si lo aceptó;
- después consulta un comprobante ya enviado: su CDR (`status`), el ticket de un resumen (`ticket`) o su estado en la
  Consulta Integrada de la SUNAT (`validity`: aceptado, anulado o no informado).

También arma, firma y envía la guía de remisión remitente (09) por la API REST de la SUNAT. Es la versión para
Objective-C del [SDK](https://github.com/Kembec/facturacion-electronica-sunat-sdk), que también funciona en .NET, Java,
JavaScript y TypeScript, Perl, PHP, Python, Ruby, Rust, Swift y la línea de comandos: los mismos datos dan el mismo
comprobante firmado en todos.

SUNAT (Peru) electronic invoicing SDK for Objective-C.

Va directo sobre la ABI C del núcleo, sin pasar por Swift: una app solo Objective-C no carga el runtime de Swift. Desde
Swift, usa el otro producto del mismo paquete, con [su README](../swift/README.md).

Tu sistema calcula los importes; el SDK los valida y los pone en el XML, pero no calcula impuestos. Guías y ejemplos:
https://fe.kembec.com

## Instalación

Se agrega por URL con Swift Package Manager, con el producto `KembecSunatSDKObjC`. En Xcode, «File › Add Package
Dependencies…» con `https://github.com/Kembec/facturacion-electronica-sunat-sdk`, la regla «Exact Version»
`1.0.0-rc.8` y ese producto; en un `Package.swift`:

```swift
// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "PrimerComprobante",
    platforms: [.macOS(.v10_15), .iOS(.v13)],
    dependencies: [
        .package(url: "https://github.com/Kembec/facturacion-electronica-sunat-sdk", exact: "1.0.0-rc.8"),
    ],
    targets: [
        .executableTarget(name: "PrimerComprobante", dependencies: [
            .product(name: "KembecSunatSDKObjC", package: "facturacion-electronica-sunat-sdk"),
        ]),
    ]
)
```

SwiftPM baja `SunatSDKFFI.xcframework.zip` del release de esa versión y lo acepta solo si su checksum es el que nombra
el `Package.swift` del tag. `exact:` te deja en la versión que probaste; `from: "1.0.0-rc.8"` tomaría también las
siguientes candidatas y la 1.0.0.

O con CocoaPods, desde Git, en tu `Podfile`:

```ruby
pod 'KembecSunatSDKObjC', :git => 'https://github.com/Kembec/facturacion-electronica-sunat-sdk.git', :tag => 'v1.0.0-rc.8'
```

El podspec no va al trunk de CocoaPods, que deja de aceptar publicaciones el 2/12/2026: se sirve desde Git, y al
instalarse baja el mismo zip del release y comprueba el mismo SHA-256 que `Package.swift` antes de usarlo.

Es la candidata 1.0.0-rc.8 (la de `KSSPackageVersion`). Corre en macOS 10.15 o posterior (Apple Silicon e Intel) y en
iOS 13 o posterior, dispositivo y simulador (arm64 y x86_64); no en Linux. Está escrito para ARC y no compila sin él.
El núcleo en Rust queda enlazado dentro de tu binario como biblioteca estática: no hay framework que embeber ni firmar,
ni programa externo al que llamar. El ejemplo de abajo, compilado en release para macOS arm64, pesa 5,4 MB y solo
enlaza Foundation, CoreFoundation, libobjc y libSystem.

Para probar una rama que aún no tiene release, clona el repositorio y descomprime el artefacto
`SunatSDKFFI.xcframework` de su run del
[workflow `swift`](https://github.com/Kembec/facturacion-electronica-sunat-sdk/actions/workflows/swift.yml) en
`bindings/swift/artifacts/SunatSDKFFI.xcframework`. Agrégalo por ruta, con SwiftPM
(`.package(path: "../facturacion-electronica-sunat-sdk")`) o con CocoaPods
(`pod 'KembecSunatSDKObjC', :path => '../facturacion-electronica-sunat-sdk'`), y corre `swift build` o `pod install`
con `KEMBEC_SUNAT_LOCAL_XCFRAMEWORK=1`. Sin esa variable, los dos toman el xcframework que nombra `release`, y
`pod install` reemplaza el que descomprimiste.

## Tu primer comprobante

Baja a la carpeta de tu proyecto la factura de prueba y el certificado de prueba del repositorio. Ese certificado es
autogenerado y no sirve para emitir.

```bash
base=https://raw.githubusercontent.com/Kembec/facturacion-electronica-sunat-sdk/v1.0.0-rc.8
curl -fsSLO "$base/fixtures/01-invoice-taxed-cash.json"
curl -fsSLO "$base/tests/vectors/test-cert/test-cert.pem"
curl -fsSLO "$base/tests/vectors/test-cert/test-key.pem"
```

```objc
#import <KembecSunatSDKObjC/KembecSunatSDKObjC.h>

int main(void) {
    @autoreleasepool {
        NSError *error = nil;
        KSSClient *sdk = [[KSSClient alloc] init];
        NSString *json = [NSString stringWithContentsOfFile:@"01-invoice-taxed-cash.json" encoding:NSUTF8StringEncoding error:&error];
        KSSSigningKey *key = [[KSSSigningKey alloc] initWithCertificatePath:@"test-cert.pem" privateKeyPath:@"test-key.pem"];
        KSSBuildResult *built = json ? [sdk buildDocumentJSON:json error:&error] : nil;
        KSSSignResult *signature = built ? [sdk signXML:built.xml key:key signatureID:nil error:&error] : nil;
        if (signature == nil) {
            NSLog(@"%@", error);
            return 1;
        }
        printf("%s %s\n", built.fileName.UTF8String, signature.digestValue.UTF8String); // 20123456789-01-F001-1 +te61a2ub5MlTFf8tKTPNGWEXKI=
    }
    return 0;
}
```

Si imprime `20123456789-01-F001-1 +te61a2ub5MlTFf8tKTPNGWEXKI=` (el nombre del archivo y la huella de la firma), el
SDK funciona: es la misma salida que dan .NET, Java, JavaScript, Perl, PHP, Python, Ruby, Rust, Swift y la CLI. Tus
propios documentos siguen la forma del
[modelo de entrada](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/input-model.md), con los
importes como texto (`"148.00"`).

## Enviar a la SUNAT y leer la respuesta

```objc
NSError *error = nil;
KSSClient *sdk = [[KSSClient alloc] initWithHTTPTimeout:60 error:&error]; // de 1 a 300 s enteros; -init usa 90 s
KSSBuildResult *built = [sdk buildDocumentJSON:documentJSON error:&error];   // el JSON de docs/input-model.md
KSSSigningKey *key = [[KSSSigningKey alloc] initWithCertificatePath:@"/seguro/cert.pem" privateKeyPath:@"/seguro/key.pem"];
KSSSignResult *signature = [sdk signXML:built.xml key:key signatureID:nil error:&error];
KSSSOLCredentials *sol = [[KSSSOLCredentials alloc] initWithUser:rucAndSOLUser password:solPassword];
KSSSendResult *sent = [sdk sendXML:signature.xml name:built.fileName credentials:sol endpoint:sendBillEndpoint error:&error];

if (sent == nil) {
    NSLog(@"%@ %@", error.userInfo[KSSErrorCommandKey], error.localizedDescription); // sin respuesta: reenvía el mismo XML
} else if (sent.outcome == KSSOutcomeFault) {
    NSLog(@"%@", sent.fault.code);                                           // por ejemplo, 1033, con su responseClass
    for (KSSDiagnosis *diagnosis in sent.diagnosis) {                        // un elemento por código de la respuesta
        NSLog(@"%@ %@ %@", diagnosis.code,                                   // 2800
              diagnosis.detail.node,                                         // lo que nombra la SUNAT: cbc:ID/schemeID = 1
              diagnosis.fields.firstObject.inputPath);                       // el campo de tu JSON: customer.document_type
    }
} else {
    [sent.cdrZip writeToFile:[built.fileName stringByAppendingString:@".cdr.zip"] atomically:YES]; // la CDR como llegó
    NSLog(@"%d", sent.cdr.isValidDocument);                                  // aceptado, con o sin observaciones
}
```

- Cada método espera a que termine su comando; los que van a la SUNAT, hasta la espera del cliente: llámalos fuera del
  hilo principal. El cliente no guarda estado después de crearse y se comparte entre hilos.
- Un método devuelve `nil` y llena `error` (dominio `KSSErrorDomain`) cuando el comando no da resultado. Un error SOAP
  de la SUNAT es un resultado (`KSSOutcomeFault`), no un error. Un envío sin respuesta es `KSSErrorCommandFailed`,
  porque pudo haber llegado: se reenvía el mismo XML firmado con el mismo número, nunca otro.
- Cada resultado de red tiene `outcome` y una propiedad por resultado posible: la que corresponde a `outcome` viene
  llena y las demás, `nil`.
- `isAccepted` es verdadero solo con código 0 y sin notas. Con código 0 y cualquier nota, sea cual sea su código, la CDR
  queda `KSSResponseClassObserved` e `isValidDocument` es verdadero en los dos casos. El 2404 (una nota que modifica un
  comprobante físico) es observación aunque esté por debajo de 4000.
- `sendSummaryXML:…` envía un resumen (RC) o una baja (RA) y devuelve su ticket: guárdalo antes que nada.
  `ticketStatus:…` y `statusForRUC:…` sin respuesta dan `KSSOutcomeNoVerdict`: se vuelve a consultar, nunca a enviar.
- `diagnosis` (en `KSSSendResult`, `KSSSummaryResult`, `KSSTicketResult` y `KSSStatusResult`) dice qué campo de tu
  entrada causó cada código: el del rechazo, o el de cada nota de una CDR observada. Cada `KSSDiagnosis` trae el `code`,
  el `detail` (nodo y valor que nombra la SUNAT; `nil` si no nombra uno) y los `fields` candidatos, cada uno con su
  `inputPath`, su `xpath` y su `certainty` (`KSSDiagnosisCertaintySource` si el nodo es el de la regla y un solo campo lo
  escribe; `…Inferred` si no). `inputPath` es `nil` cuando el modelo de entrada no tiene ese campo, y `fields` queda
  vacío si el código no está en las reglas de la SUNAT del 26.08.2026. Sin `diagnosis` en la respuesta, el arreglo es
  vacío.
- Cada resultado guarda en `rawJSON` la respuesta tal como la da el
  [contrato de la CLI](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/cli.md). Un valor de
  enumeración que esta versión no conoce se lee como `…Unknown`, no como error.

### Consultar el estado de un comprobante (Consulta Integrada)

`validityForRUC:…` pregunta a la Consulta Integrada de la SUNAT si una factura, boleta o nota (01, 03, 07, 08) existe y
en qué estado está. Usa las «Credenciales de API SUNAT» que se crean en SOL para esta consulta: no el usuario SOL ni el
par de la GRE. **No hay ambiente de pruebas**: la consulta va siempre a producción.

```objc
NSDateComponents *issueDate = [[NSDateComponents alloc] init];
issueDate.year = 2026; issueDate.month = 10; issueDate.day = 16;
KSSAPICredentials *api = [[KSSAPICredentials alloc] initWithClientID:clientID clientSecret:clientSecret];
KSSValidityResult *check = [sdk validityForRUC:@"20123456789" documentType:@"01" series:@"F001" number:123
                                     issueDate:issueDate total:[NSDecimalNumber decimalNumberWithString:@"118.00"]
                                   credentials:api requesterRUC:nil tokenEndpoint:nil endpoint:nil error:&error];
switch (check.outcome) {
    case KSSOutcomeValidity:   NSLog(@"%ld", (long)check.validity.state); break; // KSSValidityStateAccepted, …Voided, …NotReported…
    case KSSOutcomeQueryError: NSLog(@"%@", check.queryError.message); break;   // la SUNAT rechazó la consulta, no el comprobante
    default:                   NSLog(@"%@", check.noVerdict.message);           // sin respuesta en el token o la consulta: vuelve a consultar
}
```

`issueDate` toma el año, el mes y el día, e ignora lo demás; `check.validity.observations` dice qué difiere de lo que
tiene la SUNAT. `requesterRUC` es el RUC que consulta (el emisor si es `nil`), y `tokenEndpoint` y `endpoint`
reemplazan las URL de producción si no son `nil`.

Ningún resultado es motivo para emitir la operación con otro número: `KSSValidityStateNotReported` no prueba que el
comprobante no llegó (el ticket de un resumen puede seguir en proceso), así que antes se concilia con `statusForRUC:…`
o con el ticket.

### Guía de remisión remitente (09) por la API REST

`greSendXML:…` envía la guía firmada y devuelve su ticket; `greTicket:…` consulta el resultado con ese ticket. Usan el
par GRE de las «Credenciales de API SUNAT» y el usuario SOL.

```objc
KSSAPICredentials *api = [[KSSAPICredentials alloc] initWithClientID:greClientID clientSecret:greClientSecret];
KSSSOLCredentials *sol = [[KSSSOLCredentials alloc] initWithUser:rucAndSOLUser password:solPassword];
KSSGRESendResult *sent = [sdk greSendXML:signature.xml name:built.fileName apiCredentials:api solCredentials:sol
                           tokenEndpoint:tokenEndpoint endpoint:greEndpoint error:&error];

if (sent.outcome == KSSOutcomeTicket) {
    NSString *ticket = sent.greTicket.ticket;        // guárdalo antes que nada: es lo único que lleva al resultado
    KSSGRETicketResult *result = [sdk greTicket:ticket apiCredentials:api solCredentials:sol
                                  tokenEndpoint:tokenEndpoint endpoint:greEndpoint error:&error];
    // result.ticketStatus.state: KSSTicketStateInProcess (98) se vuelve a consultar con el mismo ticket;
    // …Processed (0) trae cdr; …ProcessedWithErrors (99) trae error, con su código, descripción y responseClass
    NSLog(@"%ld", (long)result.ticketStatus.state);
} else if (sent.outcome == KSSOutcomeAPIError) {
    NSLog(@"%@", sent.apiError.errors);              // lo que la SUNAT rechazó, en sus palabras
}
```

- **La SUNAT no tiene ambiente de pruebas para la GRE**, y ninguna guía de este SDK figura como aceptada por la SUNAT.
- Las dos URL base no tienen valor por defecto y solo se aceptan `https://` (las del manual:
  `https://api-seguridad.sunat.gob.pe/v1/clientessol` para el token y `https://api-cpe.sunat.gob.pe/v1` para la API).
- Si el envío no recibe respuesta, `greSendXML:…` devuelve `nil` con `KSSErrorCommandFailed`: la guía pudo haber
  llegado, así que se reenvía el mismo archivo, nunca la operación con otro número. Sin token no se envía nada y el
  resultado es `KSSOutcomeNoVerdict`.

## Errores

Todos en el dominio `KSSErrorDomain`; `userInfo[KSSErrorCommandKey]` dice qué comando falló.

| Código | Cuándo |
|---|---|
| `KSSErrorInvalidArgument` | antes de cualquier llamada: un argumento es `nil`, el texto del documento no es un valor JSON, la espera del cliente no es de 1 a 300 segundos enteros o la fecha no tiene año, mes y día |
| `KSSErrorCommandFailed` | el comando corrió y falló: el SDK rechazó el documento (el mensaje dice qué campo y por qué), falta el certificado, el endpoint no es `https://` o no hubo respuesta a un envío |
| `KSSErrorLoadFailed` | la biblioteca nativa enlazada no es la de este paquete: otra ABI u otra versión |
| `KSSErrorUnexpectedResponse` | la biblioteca respondió algo que no es la forma del contrato para ese comando |

Las contraseñas y secretos no aparecen en ningún resultado, error ni `-description`, y los bytes de la petición que los
lleva se borran después de cada llamada.

## Qué cubre y qué no

Lo mismo que el SDK en cualquier lenguaje: la lista completa está en el
[alcance del SDK](https://github.com/Kembec/facturacion-electronica-sunat-sdk#alcance) y cada campo, en el
[modelo de entrada](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/input-model.md). Lo que
el SDK no cubre lo rechaza con un error explícito en vez de armar un XML que la SUNAT rechazaría. El xcframework no
trae Mac Catalyst, tvOS, watchOS ni visionOS.

## Pruebas

Lo que aceptó la SUNAT y cómo se comprueba cada versión está en
[Cómo se comprueba](https://github.com/Kembec/facturacion-electronica-sunat-sdk#cómo-se-comprueba): la beta de la SUNAT
aceptó 36 de los 38 documentos de ejemplo; las 2 guías de remisión no tienen beta.

Para este paquete, el CI compila el núcleo como biblioteca estática para macOS (arm64 y x86_64), iOS y el simulador de
iOS (arm64 y x86_64), arma el xcframework y corre la batería enlazándolo como lo haría tu proyecto: en macOS arm64 y
x86_64 y en el simulador de iOS (como XCTest). El slice de dispositivo iOS se enlaza sin firmar; no se ejecuta. El
podspec pasa `pod lib lint` para iOS y macOS. Cada uno de los 38 documentos de
[fixtures](https://github.com/Kembec/facturacion-electronica-sunat-sdk/tree/main/fixtures) tiene que dar, con `build`
y `sign`, el XML y la firma exactos de
[tests/vectors](https://github.com/Kembec/facturacion-electronica-sunat-sdk/tree/main/tests/vectors). Además, varios
hilos firman a la vez. `statusForRUC:…`, `validityForRUC:…` y `greTicket:…` se prueban sin respuesta contra un
servidor local mudo, y `readCDR:` contra una CDR grabada; no envía nada a la SUNAT.

## Licencia

MIT, © 2026 Manuel Benancio: ver
[`LICENSE`](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/LICENSE).
