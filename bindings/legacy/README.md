# SDK de facturación electrónica SUNAT (Perú) para VB6, Visual FoxPro y PowerBuilder

Un SDK para emitir comprobantes electrónicos ante la SUNAT desde **Visual Basic 6**, **Visual FoxPro 9** o
**PowerBuilder**, para quien mantiene el sistema de facturación de una empresa en uno de ellos. Con los datos de una
factura (01), una boleta (03), una nota de crédito (07) o de débito (08), un resumen diario de boletas (RC) o una
comunicación de baja (RA):

- arma el archivo XML que pide la SUNAT y lo firma con tu certificado digital;
- lo comprime, lo envía y lee la CDR, la constancia con la que la SUNAT dice si lo aceptó;
- después consulta un comprobante ya enviado: su CDR (`status`), el ticket de un resumen (`ticket`) o su estado en la
  Consulta Integrada de la SUNAT (`validity`).

También arma, firma y envía la guía de remisión remitente (09) por la API REST de la SUNAT. Es una DLL y un módulo de
código por lenguaje, sobre la [ABI C](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/c-abi.md)
del [SDK](https://github.com/Kembec/facturacion-electronica-sunat-sdk): los mismos datos dan el mismo comprobante
firmado que en .NET, Java, Ruby, JavaScript, PHP, Python, Rust y la línea de comandos.

SUNAT (Peru) electronic invoicing SDK for VB6, Visual FoxPro and PowerBuilder.

Tu sistema calcula los importes; el SDK los valida y los pone en el XML, pero no calcula impuestos. Guías y ejemplos:
https://fe.kembec.com

**Estado: los módulos de VB6, Visual FoxPro y PowerBuilder no se han compilado ni ejecutado en sus IDE.** Lo que se
prueba en cada cambio, en Windows, es la DLL llamada con exactamente las mismas declaraciones (tipos y convención) que
traen estos archivos, y el lector de JSON de VB6 ([Pruebas](#pruebas)). Si en tu IDE algo no compila o no da la
salida esperada, abre un [issue](https://github.com/Kembec/facturacion-electronica-sunat-sdk/issues) con el mensaje.

## Qué trae el zip

Todavía no hay versión publicada: el CI arma el zip en cada cambio. Baja el artefacto
`sunat-sdk-legacy-windows-<versión>` de una ejecución del
[workflow `legacy`](https://github.com/Kembec/facturacion-electronica-sunat-sdk/actions/workflows/legacy.yml) y
descomprímelo. No hay instalador, ni registro de COM, ni runtime de Visual C++ que instalar: la DLL lleva el runtime
de C dentro.

| Ruta | Qué es |
|---|---|
| `dll\x86\sunat_sdk_ffi.dll` | la DLL de 32 bits: VB6, Visual FoxPro y PowerBuilder de 32 bits |
| `dll\x64\sunat_sdk_ffi.dll` | la de 64 bits: PowerBuilder de 64 bits |
| `vb6\` | `SunatSdk.bas` (las llamadas), `SunatJson.bas` (leer y escribir JSON), y `Sample.bas` con `Sample.vbp`, el ejemplo |
| `vfp\` | `sunat_sdk.prg` (llamadas y JSON) y `sample.prg`, el ejemplo |
| `powerbuilder\` | `n_sunat_sdk.sru` (objeto no visual) y `f_sunat_sdk_sample.srf`, el ejemplo. Usan las clases `JSONParser` y `JSONGenerator` de PowerBuilder |
| `sample\` | la factura de prueba, su XML firmado y el certificado de prueba, que es autogenerado y no sirve para emitir |

En la raíz van este README, `LICENSE` y `NOTICE`.

## Tu primer comprobante

Cada ejemplo arma la factura de `sample\`, la firma con el certificado de prueba y muestra el nombre del archivo y la
huella de la firma:

- **VB6**: abre `vb6\Sample.vbp` y ejecútalo (F5). Lo muestra en un `MsgBox`.
- **Visual FoxPro**: `DO vfp\sample.prg`. Lo muestra en un `MESSAGEBOX`.
- **PowerBuilder**: importa los dos archivos de `powerbuilder\` en una PBL y llama a
  `f_sunat_sdk_sample("C:\ruta\del\zip\")`, con la barra final. Devuelve el texto y carga la DLL de los bits del
  proceso.

La salida esperada es `20123456789-01-F001-1 +te61a2ub5MlTFf8tKTPNGWEXKI=`, la misma que dan los demás lenguajes. El
CI la obtiene por las declaraciones de estos módulos; los ejemplos en sí no se han ejecutado en su IDE.

## En tu sistema

Cada llamada es un comando de la
[línea de comandos](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/cli.md) escrito como
`{"command", "flags", "input"}`, y devuelve el JSON que imprimiría la CLI. En VB6, armar, firmar y enviar:

```vb
SunatLoad App.Path & "\dll\x86\sunat_sdk_ffi.dll"            ' una vez, al iniciar; ruta completa
built = SunatCall("{""command"":""build"",""input"":" & ReadUtf8File(documentPath) & "}")
signed = SunatCall("{""command"":""sign"",""flags"":{""cert"":" & JsonQuote(certPath) & _
    ",""key"":" & JsonQuote(keyPath) & "},""input"":{""xml"":" & JsonGet(built, "xml") & "}}")
sent = SunatCall("{""command"":""send"",""flags"":{""endpoint"":" & JsonQuote(sendBillEndpoint) & _
    ",""user"":" & JsonQuote(rucAndSolUser) & ",""password"":" & JsonQuote(solPassword) & ",""timeout"":60}," & _
    """input"":{""name"":" & JsonQuote(JsonText(built, "file_name")) & ",""xml"":" & JsonGet(signed, "xml") & "}}")

Select Case JsonText(sent, "outcome")
Case "cdr"
    cdr = JsonGet(sent, "cdr")
    code = JsonText(cdr, "response_code")      ' "0" aceptado; class: accepted, observed, rejected...
    notes = JsonGet(cdr, "notes")              ' arreglo: JsonCount(notes), JsonDecode(JsonItem(notes, 0))
Case "fault"
    code = JsonText(sent, "code")              ' por ejemplo, "1033", con su class: already_registered
End Select

' En el rechazo o la observación, "diagnosis" dice qué campo de tu JSON de entrada causó cada código:
diagnosis = JsonGet(sent, "diagnosis")         ' arreglo, un elemento por código; "" si la respuesta no lo trae
first = JsonItem(diagnosis, 0)
fields = JsonGet(first, "fields")              ' arreglo de candidatos
inputPath = JsonText(JsonItem(fields, 0), "input_path")  ' "customer.document_type" con el 2800 de la beta
```

- `JsonGet` devuelve el valor crudo, ya escrito como cadena JSON: el XML pasa de `build` a `sign` y de `sign` a `send`
  sin decodificarlo ni volver a escaparlo. `JsonText` lo decodifica, para guardarlo o mostrarlo.
- La DLL no lee variables de entorno: el endpoint, el usuario y la clave SOL y las rutas del certificado van en
  `flags`. Solo se acepta `https://`.
- Un comando que va a la SUNAT bloquea el hilo hasta su respuesta o su `timeout` (de 1 a 300 s; 90 si no se indica).
  En VB6 y Visual FoxPro es el hilo de la interfaz: el formulario no responde mientras tanto.
- `summary`, `ticket`, `status`, `validity`, `gre-send` y `gre-ticket` van igual; cada `input` y cada respuesta están en
  la [línea de comandos](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/cli.md) y cada
  campo del documento, en el
  [modelo de entrada](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/input-model.md).

En Visual FoxPro es igual con `SET PROCEDURE TO sunat_sdk.prg ADDITIVE`, `sunat_load()`, `sunat_call()` y las
funciones `sunat_json_*`; en PowerBuilder, con `n_sunat_sdk.of_load()` y `of_call()`, y el `JSONParser` y el
`JSONGenerator` del propio PowerBuilder.

## Lo que cuesta en estos lenguajes, ya resuelto

### Texto: UTF-8 en la DLL

La DLL lee y escribe UTF-8. VB6 y PowerBuilder guardan el texto en UTF-16 y Visual FoxPro en la página de códigos de
Windows. Un texto pasado en ANSI no se lee distinto: una `ñ` de Latin-1 hace que la DLL rechace la petición con
`request is not UTF-8`.

| | La petición y la respuesta | Archivos y datos de tu sistema |
|---|---|---|
| VB6 | `SunatCall` convierte con `WideCharToMultiByte` y pasa un arreglo `Byte`, nunca `As String`; la respuesta vuelve con `MultiByteToWideChar` | `ReadUtf8File` lee un archivo UTF-8 (con BOM o sin él); `Open … For Input` lo leería en ANSI |
| Visual FoxPro | todo lo de `sunat_sdk.prg` es UTF-8, que VFP guarda tal cual | `sunat_to_utf8()` para lo que viene de una tabla, un formulario o una ruta; `sunat_from_utf8()` para mostrarlo. `FILETOSTR` y `STRTOFILE` dejan los bytes como están |
| PowerBuilder | `of_call` pasa `Blob(texto, EncodingUTF8!)` y lee con `String(blob, EncodingUTF8!)` | `FileReadEx` a un `blob` y `String(blob, EncodingUTF8!)`, como en el ejemplo |

El XML firmado se guarda en UTF-8, byte por byte: pasarlo por la página de códigos pierde cualquier carácter que no
esté en ella, y con él la firma. En Visual FoxPro, `STRTOFILE(sunat_json_text(lcSigned, "xml"), …)` lo deja bien; en
VB6, `Print #` lo escribiría en ANSI, y `SunatSdk.bas` no trae una función para escribir UTF-8.

### Memoria: la libera el módulo

La DLL reserva la respuesta y solo `sunat_sdk_free` la libera. Ninguno de estos lenguajes lee un puntero, así que cada
módulo mide la respuesta con `sunat_sdk_response_len`, la copia a su propia variable con `sunat_sdk_response_copy` y
la libera enseguida, también si la copia falla. Tú no liberas nada; si escribes tus propias declaraciones, sigue ese
orden.

### Bits

Un proceso solo carga la DLL de sus propios bits. VB6 y Visual FoxPro son de 32 bits: siempre `dll\x86`. En
PowerBuilder cuentan los bits del proceso que corre, el IDE o tu ejecutable, que pueden no ser los mismos; elige la
carpeta con `ProcessBitness`, como `f_sunat_sdk_sample`. Las declaraciones de `n_sunat_sdk` usan `longptr` y sirven a
las dos.

`SunatLoad`, `sunat_load()` y `of_load()` cargan la DLL desde la ruta completa que les das y rechazan una de otra
versión de la ABI: no hace falta copiarla junto a `VB6.EXE`, al IDE ni al `PATH`.

### JSON sin componentes

VB6 no trae lector de JSON, y uno por COM es otra cosa que registrar en cada máquina. `SunatJson.bas` y su traducción
en `sunat_sdk.prg` leen los valores como texto y los decodifican al pedirlos, así que un valor anidado se lee en dos
pasos: `JsonText(JsonGet(sent, "cdr"), "response_code")`.

| | Leer | Escribir |
|---|---|---|
| VB6 | `JsonGet` (valor crudo), `JsonText` (decodificado), `JsonCount` y `JsonItem` (arreglos, desde 0), `JsonDecode` | `JsonQuote(texto)` |
| Visual FoxPro | `sunat_json_get`, `sunat_json_text`, `sunat_json_count`, `sunat_json_item`, `sunat_json_decode` | `sunat_json_quote` |
| PowerBuilder | `JSONParser` | `JSONGenerator` |

`JsonText` devuelve `""` tanto para `null` como para un miembro que no está; `JsonGet` los distingue (`"null"` o `""`).
La CDR llega también como zip en base64 (`cdr_zip_base64`), y estos módulos no la decodifican.

## Errores

Un comando que el SDK rechaza (un dato que falta, un certificado que no abre, un endpoint que no es `https://`) levanta
un error con el mensaje del SDK, que nunca lleva una clave ni un secreto:

| | Cómo llega |
|---|---|
| VB6 | `Err.Raise` con `Err.Number = vbObjectError + 513` (`SUNAT_SDK_FAILED`) y el mensaje en `Err.Description` |
| Visual FoxPro | el error 1098, con el mensaje tras `sunat_sdk: ` |
| PowerBuilder | un `RuntimeError`, con el mensaje tras `sunat_sdk: ` |

`diagnosis` viene en la respuesta de `send`, `summary`, `ticket` y `status`, al lado de `outcome`: en un `fault` y en una
CDR con código distinto de 0 o con notas, un elemento por código con `code`, `detail` (`node` y `value`, o `null`) y
`fields` (cada uno con `input_path`, `xpath` y `certainty`). Con el 2800 de la beta, `input_path` del primer campo es
`customer.document_type` y `detail` es `cbc:ID/schemeID` = `1`. Si la respuesta no lo trae, el miembro no existe: no hay
nada que diagnosticar. Aquí no hay tipos de resultado: lees ese JSON con `JsonGet` y `JsonItem`, como el resto.

Un error SOAP de la SUNAT o una consulta sin respuesta no es un error: es el `outcome` de la respuesta (`fault`,
`no_verdict`), como en la línea de comandos. Un `send` sin respuesta sí es un error, porque el comprobante pudo haber
llegado: se consulta con `status` o se reenvía el mismo XML firmado con el mismo número, nunca la operación con otro
número.

## Qué cubre y qué no

Lo mismo que el SDK en cualquier lenguaje: la lista completa está en el
[alcance del SDK](https://github.com/Kembec/facturacion-electronica-sunat-sdk#alcance). Lo que el SDK no cubre lo
rechaza con un error explícito en vez de armar un XML que la SUNAT rechazaría.

## Pruebas

Lo que aceptó la SUNAT y cómo se comprueba cada versión está en
[Cómo se comprueba](https://github.com/Kembec/facturacion-electronica-sunat-sdk#cómo-se-comprueba).

Para este zip, el CI compila la DLL para Windows de 32 y 64 bits y lee las declaraciones de `SunatSdk.bas`,
`sunat_sdk.prg` y `n_sunat_sdk.sru` tal como vienen en el zip: las traduce a C tipo por tipo, comprueba que cada una
mida lo mismo que en la ABI y, con ellas, firma los 38 documentos de
[fixtures](https://github.com/Kembec/facturacion-electronica-sunat-sdk/tree/main/fixtures), que tienen que dar la firma
exacta de [tests/vectors](https://github.com/Kembec/facturacion-electronica-sunat-sdk/tree/main/tests/vectors): a 32 bits
para los tres y a 64 bits para PowerBuilder. A 32 bits, una declaración con otra convención de llamada detiene la
prueba. También comprueba que una petición en ANSI se rechaza, que las declaraciones de VB6 y Visual FoxPro no
compilan para 64 bits, y corre `SunatJson.bas` como VBScript sobre las 38 respuestas, comparado con el lector de JSON
de PowerShell.

Lo que no se prueba: el código de VB6, Visual FoxPro y PowerBuilder dentro de su IDE, la conversión de texto de cada
uno y el lector de JSON de Visual FoxPro, que es una traducción del de VB6. Nada de esto envía a la SUNAT.

## Licencia

MIT, © 2026 Manuel Benancio: ver `LICENSE`.

En el zip, `LICENSE` y `NOTICE` van junto a este README; en el repositorio,
[`LICENSE`](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/LICENSE).
