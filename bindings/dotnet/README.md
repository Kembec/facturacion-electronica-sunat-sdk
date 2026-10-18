# SDK de facturación electrónica SUNAT (Perú) para .NET

Un SDK para emitir comprobantes electrónicos ante la SUNAT desde **C#** o **VB.NET**, para quien programa el sistema de
facturación de una empresa. Con los datos de una factura (01), una boleta (03), una nota de crédito (07) o de débito
(08), un resumen diario de boletas (RC) o una comunicación de baja (RA):

- arma el archivo XML que pide la SUNAT y lo firma con tu certificado digital;
- lo comprime, lo envía y lee la CDR, la constancia con la que la SUNAT dice si lo aceptó;
- después consulta un comprobante ya enviado: su CDR (`Status`), el ticket de un resumen (`Ticket`) o su estado en la
  Consulta Integrada de la SUNAT (`Validity`: aceptado, anulado o no informado).

También arma, firma y envía la guía de remisión remitente (09) por la API REST de la SUNAT. Es la versión para .NET del
[SDK](https://github.com/Kembec/facturacion-electronica-sunat-sdk), que también funciona en JavaScript y TypeScript,
PHP, Python, Rust y la línea de comandos: los mismos datos dan el mismo comprobante firmado en todos.

SUNAT (Peru) electronic invoicing SDK for .NET.

Tu sistema calcula los importes; el SDK los valida y los pone en el XML, pero no calcula impuestos. Guías y ejemplos:
https://fe.kembec.com

## Instalación

Desde la próxima beta, el tag del repositorio publica el paquete en NuGet y se instala con:

```bash
dotnet add package Kembec.SunatSdk --prerelease
```

Mientras tanto, o para probar un cambio sin publicar, el CI del repositorio arma el `.nupkg` en cada cambio: baja el
artefacto `nupkg` de una ejecución del
[workflow `dotnet`](https://github.com/Kembec/facturacion-electronica-sunat-sdk/actions/workflows/dotnet.yml),
descomprímelo en `nupkg/` y agrega esa carpeta como origen:

```bash
dotnet nuget add source "$PWD/nupkg" --name kembec-sunat-sdk
dotnet add package Kembec.SunatSdk --prerelease
```

Es una versión beta. El paquete trae un destino para .NET Framework 4.6.2 o posterior, otro para .NET 10 o posterior y
otro para .NET Standard 2.0, que toman .NET Core 3.1 a .NET 9. Lleva el núcleo en Rust del SDK ya compilado: no llama a
ningún programa externo ni necesita instalar nada más. Corre en Windows de 64 y 32 bits, Linux x64 y ARM64 con glibc
2.17 o posterior (no Alpine) y macOS Intel y Apple Silicon.

En .NET Framework, al compilar quedan junto a tu aplicación `runtimes\win-x64\native\` y `runtimes\win-x86\native\`, y
el paquete carga la de la arquitectura del proceso: la misma aplicación corre en 32 y en 64 bits. En un sitio ASP.NET
esa carpeta `runtimes` va dentro de `bin`, como la deja la publicación.

## Tu primer comprobante

Baja a la carpeta de tu proyecto la factura de prueba y el certificado de prueba del repositorio. Ese certificado es
autogenerado y no sirve para emitir.

```bash
base=https://raw.githubusercontent.com/Kembec/facturacion-electronica-sunat-sdk/v1.0.0-rc.2
curl -fsSLO "$base/fixtures/01-invoice-taxed-cash.json"
curl -fsSLO "$base/tests/vectors/test-cert/test-cert.pem"
curl -fsSLO "$base/tests/vectors/test-cert/test-key.pem"
```

En C#:

```csharp
using System;
using System.IO;
using Kembec.SunatSdk;

var sdk = new SunatSdkClient();
var built = sdk.Build(File.ReadAllText("01-invoice-taxed-cash.json"));
var signed = sdk.Sign(built.Xml, new SigningKey("test-cert.pem", "test-key.pem"));
Console.WriteLine(built.FileName + " " + signed.DigestValue); // 20123456789-01-F001-1 +te61a2ub5MlTFf8tKTPNGWEXKI=
```

En VB.NET:

```vb
Imports System.IO
Imports Kembec.SunatSdk

Module PrimerComprobante
    Sub Main()
        Dim sdk As New SunatSdkClient()
        Dim built = sdk.Build(File.ReadAllText("01-invoice-taxed-cash.json"))
        Dim signed = sdk.Sign(built.Xml, New SigningKey("test-cert.pem", "test-key.pem"))
        Console.WriteLine(built.FileName & " " & signed.DigestValue) ' 20123456789-01-F001-1 +te61a2ub5MlTFf8tKTPNGWEXKI=
    End Sub
End Module
```

Si imprime `20123456789-01-F001-1 +te61a2ub5MlTFf8tKTPNGWEXKI=` (el nombre del archivo y la huella de la firma), el
SDK funciona: es la misma salida que dan JavaScript, PHP, Python, Rust y la CLI. Tus propios documentos siguen la forma
del [modelo de entrada](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/input-model.md).

## Enviar a la SUNAT y leer la respuesta

```csharp
var sdk = new SunatSdkClient(TimeSpan.FromSeconds(60));          // espera a la SUNAT: de 1 a 300 s enteros; 90 s si no se indica
var built = sdk.Build(documentJson);                               // el JSON de docs/input-model.md
var signed = sdk.Sign(built.Xml, new SigningKey(@"C:\seguro\cert.pem", @"C:\seguro\key.pem"));
var sent = sdk.Send(built.FileName, signed.Xml, new SolCredentials(rucAndSolUser, solPassword), sendBillEndpoint);

if (sent.Outcome == Outcome.Fault)
{
    var code = sent.Fault!.Code;                                   // por ejemplo, "1033", con su Class
}
else
{
    File.WriteAllBytes(built.FileName + ".cdr.zip", sent.CdrZip!); // guarda la CDR como llegó
    var valid = sent.Cdr!.IsValidDocument;                         // aceptado, con o sin observaciones
}
```

- Cada método espera a que termine su comando. Los que van a la SUNAT pueden tardar hasta la espera del cliente: no
  los llames desde el hilo de una ventana. El cliente no guarda estado y se puede compartir entre hilos.
- Un error SOAP de la SUNAT es un resultado (`Outcome.Fault`), no una excepción. Un envío sin respuesta lanza
  `SunatSdkException`, porque pudo haber llegado: se reenvía el mismo XML firmado con el mismo número, nunca otro.
- `IsAccepted` es verdadero solo con código 0 y sin notas. Con código 0 y cualquier nota, sea cual sea su código, la CDR
  queda `Observed` e `IsValidDocument` es verdadero en los dos casos. El 2404 (una nota que modifica un comprobante
  físico) es observación aunque esté por debajo de 4000.
- `Summary` envía un resumen (RC) o una baja (RA) y devuelve su ticket: guárdalo antes que nada. `Ticket` y `Status`
  sin respuesta dan `Outcome.NoVerdict`: se vuelve a consultar, nunca a enviar.
- `Diagnosis` (en `SendResult`, `SummaryResult`, `TicketResult` y `StatusResult`) dice qué campo de tu JSON de entrada
  causó cada código de la respuesta: `sent.Diagnosis[0].Fields[0].InputPath` es, para el 2800 de un tipo de documento
  del receptor no permitido, `customer.document_type`, y `Detail` trae el nodo y el valor que la SUNAT nombra
  (`cbc:ID/schemeID` = `1`). Sin códigos que diagnosticar, o sin campos para el código, la lista viene vacía.
- Cada resultado guarda en `RawJson` la respuesta tal como la da el
  [contrato de la CLI](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/cli.md).

### Consultar el estado de un comprobante (Consulta Integrada)

`Validity` pregunta a la Consulta Integrada de la SUNAT si una factura, boleta o nota (01, 03, 07, 08) existe y en qué
estado está. Usa las «Credenciales de API SUNAT» que se crean en SOL para esta consulta: no el usuario SOL ni el par de
la GRE. **No hay ambiente de pruebas**: la consulta va siempre a producción.

```vb
Dim check = sdk.Validity("20123456789", "01", "F001", 123, New DateTime(2026, 10, 16), 118.00D,
                         New ApiCredentials(clientId, clientSecret))
Select Case check.Outcome
    Case Outcome.Validity
        Console.WriteLine(check.State)          ' Accepted, Voided, NotReported…; Observations dice qué difiere
    Case Outcome.QueryError
        Console.WriteLine(check.QueryError.Message) ' la SUNAT rechazó la consulta, no el comprobante
    Case Else
        Console.WriteLine(check.NoVerdict.Stage)  ' sin respuesta en Token o Query: vuelve a consultar
End Select
```

Ningún resultado es motivo para emitir la operación con otro número: `NotReported` no prueba que el comprobante no
llegó (el ticket de un resumen puede seguir en proceso), así que antes se concilia con `Status` o con el ticket.

### Guía de remisión remitente (09) por la API REST

`GreSend` envía la guía firmada y devuelve su ticket; `GreTicket` consulta el resultado con ese ticket. Usan el par GRE
de las «Credenciales de API SUNAT» y el usuario SOL.

```csharp
var api = new ApiCredentials(greClientId, greClientSecret);      // el par de GRE de «Credenciales de API SUNAT»
var sol = new SolCredentials(rucAndSolUser, solPassword);
var sent = sdk.GreSend(built.FileName, signed.Xml, api, sol, tokenEndpoint, greEndpoint);

if (sent.Outcome == Outcome.Ticket)
{
    var ticket = sent.Ticket!;                                    // guárdalo antes que nada: es lo único que lleva al resultado
    var status = sdk.GreTicket(ticket, api, sol, tokenEndpoint, greEndpoint);
    // status.State: InProcess (98) se vuelve a consultar con el mismo ticket; Processed (0) trae Cdr;
    // ProcessedWithErrors (99) trae Error, con su código, descripción y clase
}
else if (sent.Outcome == Outcome.ApiError)
{
    var errors = sent.ApiError!.Errors;                           // lo que la SUNAT rechazó, en sus palabras
}
```

- **La SUNAT no tiene ambiente de pruebas para la GRE**, y ninguna guía de este SDK figura como aceptada por la SUNAT.
- Las dos URL base no tienen valor por defecto y solo se aceptan `https://` (las del manual:
  `https://api-seguridad.sunat.gob.pe/v1/clientessol` para el token y `https://api-cpe.sunat.gob.pe/v1` para la API).
- Si el envío no recibe respuesta, `GreSend` lanza `SunatSdkException`: la guía pudo haber llegado, así que se reenvía
  el mismo archivo, nunca la operación con otro número. Sin token no se envía nada y el resultado es `Outcome.NoVerdict`.

## Errores

| Excepción | Cuándo |
|---|---|
| `ArgumentException` | el JSON del documento no es un valor JSON, falta un argumento o la espera del cliente no es de 1 a 300 segundos enteros |
| `SunatSdkException` | el comando no corrió: el SDK rechazó el documento (el mensaje dice qué campo y por qué), falta el certificado, el endpoint no es `https://`, no hubo respuesta a un envío o la respuesta no es la del contrato; `Command` dice qué comando |
| `SunatSdkLoadException` | subclase de la anterior: no se encontró la biblioteca nativa de esta plataforma, es de otra arquitectura que el proceso o de otra versión del paquete |

Las contraseñas y secretos no aparecen en ningún resultado, excepción ni `ToString()`.

## Qué cubre y qué no

Lo mismo que el SDK en cualquier lenguaje: la lista completa está en el
[alcance del SDK](https://github.com/Kembec/facturacion-electronica-sunat-sdk#alcance) y cada campo, en el
[modelo de entrada](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/input-model.md). Lo que
el SDK no cubre lo rechaza con un error explícito en vez de armar un XML que la SUNAT rechazaría.

## Pruebas

Lo que aceptó la SUNAT y cómo se comprueba cada versión está en
[Cómo se comprueba](https://github.com/Kembec/facturacion-electronica-sunat-sdk#cómo-se-comprueba): la beta de la SUNAT
aceptó 36 de los 38 documentos de ejemplo; las 2 guías de remisión no tienen beta.

Para este paquete, el CI compila el núcleo para las seis plataformas, arma el paquete y, en cada una, restaura ese
`.nupkg` como lo haría tu proyecto y corre la batería en C# y en VB.NET: cada uno de los 38 documentos de
[fixtures](https://github.com/Kembec/facturacion-electronica-sunat-sdk/tree/main/fixtures) tiene que dar, con `Build`
y `Sign`, el XML y la firma exactos de
[tests/vectors](https://github.com/Kembec/facturacion-electronica-sunat-sdk/tree/main/tests/vectors). En Windows la
corre también en .NET Framework 4.8, a 64 y a 32 bits. `Status`, `Validity` y `GreTicket` se prueban sin respuesta
contra un servidor local mudo, y `ReadCdr` contra CDR grabadas; no envía nada a la SUNAT.

## Licencia

MIT, © 2026 Manuel Benancio: ver
[`LICENSE`](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/LICENSE).
