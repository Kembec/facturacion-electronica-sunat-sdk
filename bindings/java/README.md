# SDK de facturación electrónica SUNAT (Perú) para Java

Un SDK para emitir comprobantes electrónicos ante la SUNAT desde **Java**, para quien programa el sistema de facturación
de una empresa. Con los datos de una factura (01), una boleta (03), una nota de crédito (07) o de débito (08), un
resumen diario de boletas (RC) o una comunicación de baja (RA):

- arma el archivo XML que pide la SUNAT y lo firma con tu certificado digital;
- lo comprime, lo envía y lee la CDR, la constancia con la que la SUNAT dice si lo aceptó;
- después consulta un comprobante ya enviado: su CDR (`status`), el ticket de un resumen (`ticket`) o su estado en la
  Consulta Integrada de la SUNAT (`validity`: aceptado, anulado o no informado).

También arma, firma y envía la guía de remisión remitente (09) por la API REST de la SUNAT. Es la versión para Java del
[SDK](https://github.com/Kembec/facturacion-electronica-sunat-sdk), que también funciona en .NET, JavaScript y
TypeScript, PHP, Python, Rust y la línea de comandos: los mismos datos dan el mismo comprobante firmado en todos.

SUNAT (Peru) electronic invoicing SDK for Java.

Tu sistema calcula los importes; el SDK los valida y los pone en el XML, pero no calcula impuestos. Guías y ejemplos:
https://fe.kembec.com

## Instalación

El paquete todavía no está en Maven Central. Cuando se publique, se agregará con Maven:

```xml
<dependency>
  <groupId>com.kembec</groupId>
  <artifactId>sunat-sdk</artifactId>
  <version>0.1.0-beta.4</version>
</dependency>
```

o con Gradle: `implementation("com.kembec:sunat-sdk:0.1.0-beta.4")`.

Hasta entonces, el CI del repositorio arma el jar en cada cambio: baja el artefacto `maven` de una ejecución del
[workflow `java`](https://github.com/Kembec/facturacion-electronica-sunat-sdk/actions/workflows/java.yml),
descomprímelo e instala el jar con su pom en tu repositorio local de Maven; después, la dependencia de arriba resuelve
desde ahí (en Gradle, con `mavenLocal()` en `repositories`):

```bash
mvn org.apache.maven.plugins:maven-install-plugin:3.2.0:install-file \
  -Dfile=sunat-sdk-0.1.0-beta.4.jar -DpomFile=sunat-sdk-0.1.0-beta.4.pom
```

Es una versión beta. Corre en Java 8 o posterior (probado en 8, 11, 17, 21 y 25) y trae dentro del jar el núcleo en
Rust del SDK ya compilado: no llama a ningún programa externo ni necesita instalar nada más. Su única dependencia es
[JNA](https://github.com/java-native-access/jna). Corre en Windows de 64 y 32 bits (en 32 bits, con Java 8, 11 o 17),
Linux x64 y ARM64 con glibc 2.17 o posterior (no Alpine) y macOS Intel y Apple Silicon.

Al primer uso, JNA copia la biblioteca de tu plataforma a una carpeta temporal y la carga desde ahí. Si tu servidor
monta esa carpeta sin permiso de ejecución, indica otra con `-Djna.tmpdir=/ruta`, o apunta a la biblioteca con
`-Dkembec.sunatsdk.library=/ruta/libsunat_sdk_ffi.so`.

## Tu primer comprobante

Baja a la carpeta de tu proyecto la factura de prueba y el certificado de prueba del repositorio. Ese certificado es
autogenerado y no sirve para emitir.

```bash
base=https://raw.githubusercontent.com/Kembec/facturacion-electronica-sunat-sdk/v0.1.0-beta.4
curl -fsSLO "$base/fixtures/01-invoice-taxed-cash.json"
curl -fsSLO "$base/tests/vectors/test-cert/test-cert.pem"
curl -fsSLO "$base/tests/vectors/test-cert/test-key.pem"
```

```java
import com.kembec.sunatsdk.*;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;

public class PrimerComprobante {
    public static void main(String[] args) throws Exception {
        SunatSdkClient sdk = new SunatSdkClient();
        String json = new String(Files.readAllBytes(Paths.get("01-invoice-taxed-cash.json")), StandardCharsets.UTF_8);
        BuildResult built = sdk.build(json);
        SignResult signed = sdk.sign(built.getXml(), new SigningKey("test-cert.pem", "test-key.pem"));
        System.out.println(built.getFileName() + " " + signed.getDigestValue()); // 20123456789-01-F001-1 +te61a2ub5MlTFf8tKTPNGWEXKI=
    }
}
```

Si imprime `20123456789-01-F001-1 +te61a2ub5MlTFf8tKTPNGWEXKI=` (el nombre del archivo y la huella de la firma), el
SDK funciona: es la misma salida que dan .NET, JavaScript, PHP, Python, Rust y la CLI. Tus propios documentos siguen la
forma del [modelo de entrada](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/input-model.md).

## Enviar a la SUNAT y leer la respuesta

```java
SunatSdkClient sdk = new SunatSdkClient(Duration.ofSeconds(60));    // espera a la SUNAT: de 1 a 300 s enteros; 90 s si no se indica
BuildResult built = sdk.build(documentJson);                         // el JSON de docs/input-model.md
SignResult signed = sdk.sign(built.getXml(), new SigningKey("/seguro/cert.pem", "/seguro/key.pem"));
SendResult sent = sdk.send(built.getFileName(), signed.getXml(), new SolCredentials(rucAndSolUser, solPassword), sendBillEndpoint);

if (sent.getOutcome() == Outcome.FAULT) {
    String code = sent.getFault().getCode();                         // por ejemplo, "1033", con su getResponseClass()
} else {
    Files.write(Paths.get(built.getFileName() + ".cdr.zip"), sent.getCdrZip()); // guarda la CDR como llegó
    boolean valid = sent.getCdr().isValidDocument();                 // aceptado, con o sin observaciones
}
```

- Cada método espera a que termine su comando. Los que van a la SUNAT pueden tardar hasta la espera del cliente: no
  los llames desde el hilo de una ventana. El cliente no guarda estado y se puede compartir entre hilos.
- Un error SOAP de la SUNAT es un resultado (`Outcome.FAULT`), no una excepción. Un envío sin respuesta lanza
  `SunatSdkException`, porque pudo haber llegado: se reenvía el mismo XML firmado con el mismo número, nunca otro.
- `isAccepted()` es verdadero solo con código 0 y sin observaciones; con código 0 y una nota de 4000 o más, la CDR
  queda `OBSERVED` e `isValidDocument()` es verdadero en los dos casos.
- `summary` envía un resumen (RC) o una baja (RA) y devuelve su ticket: guárdalo antes que nada. `ticket` y `status`
  sin respuesta dan `Outcome.NO_VERDICT`: se vuelve a consultar, nunca a enviar.
- Cada resultado guarda en `getRawJson()` la respuesta tal como la da el
  [contrato de la CLI](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/cli.md).

### Consultar el estado de un comprobante (Consulta Integrada)

`validity` pregunta a la Consulta Integrada de la SUNAT si una factura, boleta o nota (01, 03, 07, 08) existe y en qué
estado está. Usa las «Credenciales de API SUNAT» que se crean en SOL para esta consulta: no el usuario SOL ni el par de
la GRE. **No hay ambiente de pruebas**: la consulta va siempre a producción.

```java
ValidityResult check = sdk.validity("20123456789", "01", "F001", 123, LocalDate.of(2026, 10, 16),
        new BigDecimal("118.00"), new ApiCredentials(clientId, clientSecret));
switch (check.getOutcome()) {
    case VALIDITY:    System.out.println(check.getState()); break;                   // ACCEPTED, VOIDED, NOT_REPORTED…; getObservations() dice qué difiere
    case QUERY_ERROR: System.out.println(check.getQueryError().getMessage()); break; // la SUNAT rechazó la consulta, no el comprobante
    default:          System.out.println(check.getNoVerdict().getStage());           // sin respuesta en TOKEN o QUERY: vuelve a consultar
}
```

Ningún resultado es motivo para emitir la operación con otro número: `NOT_REPORTED` no prueba que el comprobante no
llegó (el ticket de un resumen puede seguir en proceso), así que antes se concilia con `status` o con el ticket.

### Guía de remisión remitente (09) por la API REST

`greSend` envía la guía firmada y devuelve su ticket; `greTicket` consulta el resultado con ese ticket. Usan el par GRE
de las «Credenciales de API SUNAT» y el usuario SOL.

```java
ApiCredentials api = new ApiCredentials(greClientId, greClientSecret);   // el par de GRE de «Credenciales de API SUNAT»
SolCredentials sol = new SolCredentials(rucAndSolUser, solPassword);
GreSendResult sent = sdk.greSend(built.getFileName(), signed.getXml(), api, sol, tokenEndpoint, greEndpoint);

if (sent.getOutcome() == Outcome.TICKET) {
    String ticket = sent.getTicket();                                     // guárdalo antes que nada: es lo único que lleva al resultado
    GreTicketResult status = sdk.greTicket(ticket, api, sol, tokenEndpoint, greEndpoint);
    // status.getState(): IN_PROCESS (98) se vuelve a consultar con el mismo ticket; PROCESSED (0) trae getCdr();
    // PROCESSED_WITH_ERRORS (99) trae getError(), con su código, descripción y clase
} else if (sent.getOutcome() == Outcome.API_ERROR) {
    List<GreApiErrorItem> errors = sent.getApiError().getErrors();        // lo que la SUNAT rechazó, en sus palabras
}
```

- **La SUNAT no tiene ambiente de pruebas para la GRE**, y ninguna guía de este SDK figura como aceptada por la SUNAT.
- Las dos URL base no tienen valor por defecto y solo se aceptan `https://` (las del manual:
  `https://api-seguridad.sunat.gob.pe/v1/clientessol` para el token y `https://api-cpe.sunat.gob.pe/v1` para la API).
- Si el envío no recibe respuesta, `greSend` lanza `SunatSdkException`: la guía pudo haber llegado, así que se reenvía
  el mismo archivo, nunca la operación con otro número. Sin token no se envía nada y el resultado es
  `Outcome.NO_VERDICT`.

## Errores

| Excepción | Cuándo |
|---|---|
| `IllegalArgumentException` | el JSON del documento no es un valor JSON, o la espera del cliente no es de 1 a 300 segundos enteros |
| `NullPointerException` | falta un argumento obligatorio |
| `SunatSdkException` | el comando no corrió: el SDK rechazó el documento (el mensaje dice qué campo y por qué), falta el certificado, el endpoint no es `https://`, no hubo respuesta a un envío o la respuesta no es la del contrato; `getCommand()` dice qué comando |
| `SunatSdkLoadException` | subclase de la anterior: no hay biblioteca nativa para esta plataforma, no se pudo extraer o cargar, o es de otra versión del paquete |

Las dos del SDK son no comprobadas (`RuntimeException`). Las contraseñas y secretos no aparecen en ningún resultado,
excepción ni `toString()`, y los bytes de la petición que los lleva se borran después de cada llamada.

## Qué cubre y qué no

Lo mismo que el SDK en cualquier lenguaje: la lista completa está en el
[alcance del SDK](https://github.com/Kembec/facturacion-electronica-sunat-sdk#alcance) y cada campo, en el
[modelo de entrada](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/input-model.md). Lo que
el SDK no cubre lo rechaza con un error explícito en vez de armar un XML que la SUNAT rechazaría.

## Pruebas

Lo que aceptó la SUNAT y cómo se comprueba cada versión está en
[Cómo se comprueba](https://github.com/Kembec/facturacion-electronica-sunat-sdk#cómo-se-comprueba): la beta de la SUNAT
aceptó 36 de los 38 documentos de ejemplo; las 2 guías de remisión no tienen beta.

Para este paquete, el CI compila el núcleo para las seis plataformas, arma el jar y, en cada una, lo instala en un
repositorio de Maven propio, lo resuelve como lo haría tu proyecto y corre la batería en Java 8, 11, 17, 21 y 25 (en
Windows de 32 bits, 8, 11 y 17, y falla si la JVM no es de 32 bits): cada uno de los 38 documentos de
[fixtures](https://github.com/Kembec/facturacion-electronica-sunat-sdk/tree/main/fixtures) tiene que dar, con `build`
y `sign`, el XML y la firma exactos de
[tests/vectors](https://github.com/Kembec/facturacion-electronica-sunat-sdk/tree/main/tests/vectors). En Linux la corre
además con Java 8 en un sistema con glibc 2.17. `status`, `validity` y `greTicket` se prueban sin respuesta contra un
servidor local mudo, y `readCdr` contra una CDR grabada; no envía nada a la SUNAT.

## Licencia

MIT, © 2026 Manuel Benancio: ver
[`LICENSE`](https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/LICENSE).
