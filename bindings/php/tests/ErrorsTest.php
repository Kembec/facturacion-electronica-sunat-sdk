<?php

declare(strict_types=1);

namespace Kembec\SunatSdk\Tests;

use Kembec\SunatSdk\Exception\BinaryNotExecutableException;
use Kembec\SunatSdk\Exception\BinaryNotFoundException;
use Kembec\SunatSdk\Exception\InvalidOutputException;
use Kembec\SunatSdk\Exception\SdkErrorException;
use Kembec\SunatSdk\Exception\TimeoutException;
use Kembec\SunatSdk\SolCredentials;
use Kembec\SunatSdk\SunatSdk;

/** Each way a call can fail arrives as its own exception type. */
final class ErrorsTest extends BinaryTestCase
{
    public function testMissingBinary(): void
    {
        $sdk = new SunatSdk(sys_get_temp_dir() . '/no-such-sunat-sdk-' . uniqid());

        $this->expectException(BinaryNotFoundException::class);
        $sdk->zip('20100066603-01-F001-1', '<a/>');
    }

    public function testBinaryWithoutExecutePermission(): void
    {
        $sdk = new SunatSdk($this->fakeBinary('echo "{}"', 0644));

        $this->expectException(BinaryNotExecutableException::class);
        $sdk->zip('20100066603-01-F001-1', '<a/>');
    }

    public function testTimeoutKillsTheProcess(): void
    {
        $sdk = new SunatSdk($this->fakeBinary('sleep 5'), 0.3);
        $started = microtime(true);

        try {
            $sdk->zip('20100066603-01-F001-1', str_repeat('<a/>', 100000));
            self::fail('expected a timeout');
        } catch (TimeoutException $e) {
            self::assertLessThan(2.0, microtime(true) - $started);
            self::assertStringContainsString('zip', $e->getMessage());
        }
    }

    public function testSdkErrorCarriesTheErrorMessage(): void
    {
        $sdk = new SunatSdk(self::binaryPath());

        try {
            $sdk->zip('bad', '<a/>');
            self::fail('expected an SDK error');
        } catch (SdkErrorException $e) {
            self::assertNotSame('', $e->getMessage());
            self::assertStringNotContainsString('{"error"', $e->getMessage());
            self::assertSame('zip', $e->getCommand());
            self::assertSame(1, $e->getExitCode());
        }
    }

    public function testUnreadableCdrIsAnSdkError(): void
    {
        $this->expectException(SdkErrorException::class);
        (new SunatSdk(self::binaryPath()))->cdr('not a cdr');
    }

    public function testRejectedEndpointNeverEchoesTheCredentials(): void
    {
        $sdk = new SunatSdk(self::binaryPath());
        $credentials = new SolCredentials('20100066603MODDATOS', 'pw-very-secret');

        try {
            $sdk->send('20100066603-01-F001-1', '<a/>', $credentials, 'http://plain.example/billService');
            self::fail('expected an SDK error');
        } catch (SdkErrorException $e) {
            self::assertSame('endpoint must use https://', $e->getMessage());
            self::assertStringNotContainsString('pw-very-secret', $e->getMessage());
            self::assertStringNotContainsString('pw-very-secret', $e->getTraceAsString());
        }
    }

    public function testMissingStatusCredentialsAreReportedByName(): void
    {
        $sdk = new SunatSdk(self::binaryPath());
        $credentials = new SolCredentials('20100066603MODDATOS', 'pw-very-secret');

        $this->expectException(SdkErrorException::class);
        $this->expectExceptionMessage('missing --endpoint or SUNAT_STATUS_ENDPOINT');
        $sdk->status('20100066603', '01', 'F001', 1, $credentials, '');
    }

    public function testSecretsEchoedOnStderrAreRedacted(): void
    {
        $script = 'printf \'{"error":"login failed for %s with %s"}\' "$SUNAT_SOL_USER" "$SUNAT_SOL_PASSWORD" >&2; exit 1';
        $sdk = new SunatSdk($this->fakeBinary($script));

        try {
            $sdk->send('20100066603-01-F001-1', '<a/>', new SolCredentials('20100066603USER', 'pw-very-secret'), 'https://e.example');
            self::fail('expected an SDK error');
        } catch (SdkErrorException $e) {
            self::assertSame('login failed for [redacted] with [redacted]', $e->getMessage());
        }
    }

    public function testNonJsonOutputIsInvalidOutput(): void
    {
        $sdk = new SunatSdk($this->fakeBinary('cat >/dev/null; echo "not json"'));

        $this->expectException(InvalidOutputException::class);
        $sdk->zip('20100066603-01-F001-1', '<a/>');
    }

    public function testOutputWithoutTheContractFieldsIsInvalidOutput(): void
    {
        $sdk = new SunatSdk($this->fakeBinary('cat >/dev/null; echo \'{"file_name": 1}\''));

        $this->expectException(InvalidOutputException::class);
        $sdk->zip('20100066603-01-F001-1', '<a/>');
    }
}
