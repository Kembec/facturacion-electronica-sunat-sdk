# SDK de facturación electrónica SUNAT (Perú) para VB6, Visual FoxPro y PowerBuilder

Un SDK para emitir comprobantes electrónicos ante la SUNAT desde un sistema hecho en **Visual Basic 6**, **Visual
FoxPro 9** o **PowerBuilder**. Con los datos de una factura (01), una boleta (03), una nota de crédito (07) o de débito
(08), un resumen diario (RC) o una comunicación de baja (RA), arma el XML que pide la SUNAT, lo firma con tu
certificado, lo envía y lee la CDR. Es la versión para estos lenguajes del
[SDK](https://github.com/Kembec/facturacion-electronica-sunat-sdk): los mismos datos dan el mismo comprobante firmado
que en .NET, Java, PHP, Python, JavaScript y la línea de comandos.

SUNAT (Peru) electronic invoicing SDK for VB6, Visual FoxPro and PowerBuilder.

**Estado: los módulos de VB6, Visual FoxPro y PowerBuilder no se han compilado en sus IDE.** Lo que sí se prueba en cada
cambio, en Windows, es la DLL llamada con exactamente las mismas declaraciones (tipos y convención) que traen estos
archivos, más el lector de JSON de VB6 ([Pruebas](#pruebas)). Si lo abres en tu IDE y algo no compila, abre un issue con
el mensaje.

## Qué trae el zip

Es una DLL y un archivo de código por lenguaje: no hay instalador, ni registro de COM, ni runtime de Visual C++.

| Carpeta | Qué es |
|---|---|
| `dll\x86\sunat_sdk_ffi.dll` | la DLL de 32 bits: VB6, Visual FoxPro y PowerBuilder de 32 bits |
| `dll\x64\sunat_sdk_ffi.dll` | la de 64 bits: PowerBuilder de 64 bits |
| `vb6\` | `SunatSdk.bas` (las llamadas), `SunatJson.bas` (leer y escribir JSON) y `Sample.vbp`, el ejemplo |
| `vfp\` | `sunat_sdk.prg` (llamadas y JSON) y `sample.prg` |
| `powerbuilder\` | `n_sunat_sdk.sru` (objeto no visual) y `f_sunat_sdk_sample.srf`, el ejemplo; para PowerBuilder 2017 R3 o posterior |
| `sample\` | la factura de prueba, su vector firmado y el certificado de prueba, que es autogenerado y no sirve para emitir |

Todavía no hay versión publicada: el CI arma el zip en cada cambio. Baja el artefacto `sunat-sdk-legacy-windows-…` de
una ejecución del [workflow `legacy`](https://github.com/Kembec/facturacion-electronica-sunat-sdk/actions/workflows/legacy.yml)
y descomprímelo.

Un proceso solo carga la DLL de sus propios bits. VB6 y Visual FoxPro son siempre de 32 bits; en PowerBuilder, la
del IDE o del runtime con el que corre tu aplicación.

## Tu primer comprobante

Cada ejemplo arma la factura de `sample\`, la firma con el certificado de prueba y muestra el nombre del archivo y la
huella de la firma. Si sale `20123456789-01-F001-1 +te61a2ub5MlTFf8tKTPNGWEXKI=`, la DLL funciona: es la misma salida
que en los demás lenguajes.

- **VB6**: abre `vb6\Sample.vbp` y ejecútalo (F5).
- **Visual FoxPro**: `DO vfp\sample.prg`.
- **PowerBuilder**: importa los dos archivos de `powerbuilder\` en una PBL y llama a
  `f_sunat_sdk_sample("C:\ruta\del\zip\")`, que devuelve ese texto.

En tu sistema, VB6 se ve así:

```vb
SunatLoad App.Path & "\dll\x86\sunat_sdk_ffi.dll"   ' una vez, al iniciar
built = SunatCall("{""command"":""build"",""input"":" & documentJson & "}")
signed = SunatCall("{""command"":""sign"",""flags"":{""cert"":" & JsonQuote(certPath) & ",""key"":" & _
    JsonQuote(keyPath) & "},""input"":{""xml"":" & JsonQuote(JsonText(built, "xml")) & "}}")
fileName = JsonText(built, "file_name")             ' 20123456789-01-F001-1
signedXml = JsonText(signed, "xml")                  ' el XML firmado, para guardarlo y enviarlo
```

Cada llamada es un comando de la [línea de comandos](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/cli.md)
con la forma `{"command", "flags", "input"}` de la
[ABI C](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/c-abi.md): `send`, `status`,
`summary`, `ticket`, `validity` y la guía de remisión van igual. La DLL no lee variables de entorno: el endpoint, el
usuario y la clave SOL van en `flags`. Un comando que va a la SUNAT espera hasta su `timeout` (90 s si no se indica):
no lo llames desde el evento de un botón sin avisar al usuario.

## Lo que cuesta en estos lenguajes, ya resuelto

**Texto: UTF-8, no ANSI.** La DLL lee y escribe UTF-8; VB6 y PowerBuilder guardan el texto en UTF-16 y Visual FoxPro en
la página de códigos de Windows. Un `As String` en un `Declare` de VB6 manda el texto en ANSI y una `ñ` llegaría mal: la
DLL la rechaza con `request is not UTF-8`, nunca la lee distinto.

| | Cómo pasa el texto |
|---|---|
| VB6 | `SunatCall` convierte con `WideCharToMultiByte` y pasa un arreglo `Byte`; la respuesta vuelve con `MultiByteToWideChar` |
| Visual FoxPro | todo el texto del SDK es UTF-8, que VFP guarda tal cual: convierte solo en los bordes, `sunat_to_utf8()` para lo que viene de una tabla o un formulario y `sunat_from_utf8()` para mostrarlo |
| PowerBuilder | `of_call` pasa `Blob(texto, EncodingUTF8!)` y lee con `String(blob, EncodingUTF8!)` |

En Visual FoxPro, el XML firmado se guarda como vino (UTF-8): pasarlo a la página de códigos y de vuelta perdería
cualquier carácter que no esté en ella, y con él la firma.

**Memoria: la libera la DLL.** La respuesta la reserva la DLL y solo `sunat_sdk_free` la libera. Estos lenguajes no
leen un puntero, así que cada módulo mide la respuesta con `sunat_sdk_response_len`, la copia a su propia variable con
`sunat_sdk_response_copy` y la libera enseguida, también si la copia falla. Tú no liberas nada.

**JSON sin componentes.** VB6 no trae lector de JSON y uno por COM es otra cosa que registrar en cada máquina:
`SunatJson.bas` (y su versión en `sunat_sdk.prg`) lee los valores como texto y los decodifica al pedirlos.

| | Leer | Escribir |
|---|---|---|
| VB6 | `JsonText(json, "clave")`, `JsonGet` para un objeto o arreglo, `JsonCount`, `JsonItem` | `JsonQuote(texto)` |
| Visual FoxPro | `sunat_json_text`, `sunat_json_get`, `sunat_json_count`, `sunat_json_item` | `sunat_json_quote` |
| PowerBuilder | `JSONParser` del propio PowerBuilder | `JSONGenerator` |

Un valor anidado se lee en dos pasos: `JsonText(JsonGet(respuesta, "cdr"), "code")`.

## Errores

Un comando que el SDK rechaza (un dato que falta, un certificado que no abre, un endpoint que no es `https://`) levanta
un error con el mensaje del SDK, que nunca lleva una clave ni un secreto: en VB6 `Err.Number = vbObjectError + 513`,
en Visual FoxPro el error 1098 y en PowerBuilder un `RuntimeError`. Un error SOAP de la SUNAT o una consulta sin
respuesta no es un error: es el `outcome` de la respuesta, como en la línea de comandos.

## Pruebas

Lo que aceptó la SUNAT y cómo se comprueba cada versión está en
[Cómo se comprueba](https://github.com/Kembec/facturacion-electronica-sunat-sdk#cómo-se-comprueba).

Para este zip, el CI compila la DLL para Windows de 32 y 64 bits y lee las declaraciones de `SunatSdk.bas`,
`sunat_sdk.prg` y `n_sunat_sdk.sru` tal como vienen en el zip: las traduce a C tipo por tipo, comprueba que cada una
mida lo mismo que en la ABI y, con ellas, firma los 38 documentos de
[fixtures](https://github.com/Kembec/facturacion-electronica-sunat-sdk/tree/main/fixtures), que tienen que dar la firma
exacta de [tests/vectors](https://github.com/Kembec/facturacion-electronica-sunat-sdk/tree/main/tests/vectors): a 32 bits
para los tres y a 64 bits para PowerBuilder. A 32 bits, una declaración con otra convención de llamada detiene la
prueba. También comprueba que las declaraciones de VB6 y Visual FoxPro no compilan para 64 bits, y corre
`SunatJson.bas` como VBScript sobre las 38 respuestas, comparado con el lector de JSON de PowerShell.

Lo que no se prueba: el código de VB6, Visual FoxPro y PowerBuilder dentro de su IDE, la conversión de texto de cada
uno y el lector de JSON de Visual FoxPro, que es una traducción del de VB6.

## Licencia

MIT, © 2026 Manuel Benancio: ver `LICENSE`.
