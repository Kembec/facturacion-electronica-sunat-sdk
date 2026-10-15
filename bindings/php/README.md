# kembec/facturacion-electronica-sunat-sdk (PHP)

PHP client for the `sunat-sdk` binary (contract: `docs/cli.md`). PHP 7.4 or later, no extensions
beyond `json`. Each call runs the binary once with `proc_open`.

```php
use Kembec\SunatSdk\SunatSdk;
use Kembec\SunatSdk\SigningKey;
use Kembec\SunatSdk\SolCredentials;

$sdk = new SunatSdk('/usr/local/bin/sunat-sdk', 120.0);     // binary path, timeout per call in seconds

$built  = $sdk->build($document);                             // array, object or JSON string (docs/input-model.md)
$signed = $sdk->sign($built->getXml(), new SigningKey('/secure/cert.pem', '/secure/key.pem'));
$result = $sdk->send($built->getFileName(), $signed->getXml(),
    new SolCredentials($rucAndSolUser, $solPassword), $sendBillEndpoint);

if ($result->isFault()) {
    $result->getFault()->getCode();                           // e.g. "1033", with getClass()
} else {
    $result->getCdr()->isAccepted();                          // response code, notes, class
}

$sdk->status($ruc, '01', 'F001', 123, $credentials, $statusEndpoint);
$sdk->zip($fileName, $xml);                                   // ZipResult
$sdk->cdr($cdrZipOrXmlBytes);                                 // Cdr
```

Secrets: SOL credentials, endpoints and key paths reach the binary only through its environment,
never argv. The child does not inherit the parent's environment, and any value passed to it is
replaced by `[redacted]` in error messages. `SolCredentials` and `SigningKey` hide their values from
`var_dump` and `print_r`.

Errors, all subclasses of `Exception\SunatSdkException`:

| Exception | When |
|---|---|
| `SdkErrorException` | the binary exited non-zero; message = its `error`, plus `getCommand()` and `getExitCode()` |
| `BinaryNotFoundException` | the path is not a file |
| `BinaryNotExecutableException` | no execute permission, or the process could not start |
| `TimeoutException` | the call exceeded the timeout; the process is killed |
| `InvalidOutputException` | exit 0 but stdout is not the JSON the contract promises |

A SOAP fault from SUNAT is a result (`isFault()`), not an exception.

## Tests

```bash
curl -sSL https://getcomposer.org/download/latest-stable/composer.phar -o composer.phar
php composer.phar install && vendor/bin/phpunit
```

They run against the real binary: `SUNAT_SDK_BIN`, or `../../target/debug/sunat-sdk` by default. The
fixtures in `fixtures/` must give, through `build` and `sign`, exactly the XML and signature in
`tests/vectors/` (signed with the self-generated test certificate). Nothing is sent to SUNAT.
