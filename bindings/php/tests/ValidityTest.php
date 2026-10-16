<?php

declare(strict_types=1);

namespace Kembec\SunatSdk\Tests;

use Kembec\SunatSdk\ApiCredentials;
use Kembec\SunatSdk\Exception\InvalidOutputException;
use Kembec\SunatSdk\Exception\SdkErrorException;
use Kembec\SunatSdk\Exception\TimeoutException;
use Kembec\SunatSdk\Result\ValidityResult;
use Kembec\SunatSdk\SunatSdk;
use PHPUnit\Framework\Attributes\DataProvider;

/**
 * `validity` (Consulta Integrada) from PHP. The binary only accepts https:// bases and the client passes
 * none, so a real call would reach SUNAT's production: each outcome is replayed by a fake binary that
 * prints what the core makes of one recorded answer in crates/sunat-sdk/tests/data/validity/ (the
 * mapping itself is tested in crates/sunat-sdk/src/validity_tests.rs). No network.
 */
final class ValidityTest extends BinaryTestCase
{
    private const CLIENT_ID = 'c04864ce-f454-48e1-b41d-8730e575b905';
    private const CLIENT_SECRET = 'secret-not-real==';

    /** @var string[] */
    private $files = [];

    /**
     * Recorded answer → the binary's stdout for it.
     *
     * @return array<string, array{string, string, ?string, string[]}>
     */
    public static function states(): array
    {
        $two = [
            '- Fecha de emisión ingresada no coincide con la registrada en la SUNAT.',
            '- Receptor del comprobante de pago NO corresponde con el número de RUC que invoca al servicio web.',
        ];

        return [
            'estadoCp 0' => ['derived-query-0', '0', ValidityResult::NOT_REPORTED, []],
            'estadoCp 1, the manual' => ['manual-query-1', '1', ValidityResult::ACCEPTED, $two],
            'estadoCp 1 as a number' => ['derived-query-1-numeric', '1', ValidityResult::ACCEPTED, ['- x']],
            'estadoCp 2' => ['derived-query-2', '2', ValidityResult::VOIDED, []],
            'estadoCp 3' => ['derived-query-3', '3', ValidityResult::PRINTED_AUTHORIZED, []],
            'estadoCp 4' => ['derived-query-4', '4', ValidityResult::PRINTED_NOT_AUTHORIZED, []],
            'estadoCp outside the manual' => ['derived-query-unknown', '7', null, []],
        ];
    }

    /**
     * @dataProvider states
     * @param string[] $observations
     */
    #[DataProvider('states')]
    public function testEachStateCode(string $recording, string $code, ?string $state, array $observations): void
    {
        $recorded = self::recording($recording);
        self::assertSame($code, (string) $recorded['data']['estadoCp'], 'the replay must match its recording');

        $result = $this->validity([
            'outcome' => 'validity',
            'state_code' => $code,
            'state' => $state,
            'taxpayer_state_code' => '00',
            'domicile_condition_code' => '00',
            'observations' => $observations,
            'message' => trim($recorded['message']),
        ]);

        self::assertTrue($result->hasState());
        self::assertFalse($result->isQueryError());
        self::assertFalse($result->isNoVerdict());
        self::assertSame($code, $result->getStateCode());
        self::assertSame($state, $result->getState());
        self::assertSame('00', $result->getTaxpayerStateCode());
        self::assertSame('00', $result->getDomicileConditionCode());
        self::assertSame($observations, $result->getObservations());
        self::assertSame('Operation Success!', $result->getMessage());
        self::assertNull($result->getErrorCode());
        self::assertNull($result->getStage());
        self::assertNull($result->getNoVerdictReason());
    }

    public function testAMalformedQueryIsAQueryErrorNotAVerdict(): void
    {
        $recorded = self::recording('manual-query-422');
        $result = $this->validity([
            'outcome' => 'query_error',
            'http_status' => 422,
            'error_code' => $recorded['errorCode'],
            'message' => trim($recorded['message']),
        ]);

        self::assertTrue($result->isQueryError());
        self::assertFalse($result->hasState());
        self::assertFalse($result->isNoVerdict());
        self::assertSame('99', $result->getErrorCode());
        self::assertSame(422, $result->getHttpStatus());
        self::assertStringStartsWith('Unexpected character', (string) $result->getMessage());
        self::assertNull($result->getState());
        self::assertNull($result->getNoVerdictReason());
    }

    /**
     * @return array<string, array{string, ?int, string}>
     */
    public static function silences(): array
    {
        return [
            'token 401' => ['token', 401, 'SUNAT answered HTTP 401 to the token request'],
            'query 401' => ['query', 401, 'SUNAT answered HTTP 401 to the query request'],
            'token timeout' => ['token', null, 'timed out after 85 s waiting for the token answer'],
            'query timeout' => ['query', null, 'timed out after 85 s waiting for the query answer'],
        ];
    }

    /**
     * @dataProvider silences
     */
    #[DataProvider('silences')]
    public function testWithoutAnAnswerThereIsNoVerdict(string $stage, ?int $httpStatus, string $message): void
    {
        $result = $this->validity([
            'outcome' => 'no_verdict',
            'stage' => $stage,
            'reason' => $httpStatus === null ? 'timeout' : 'http_status',
            'http_status' => $httpStatus,
            'message' => $message,
        ]);

        self::assertTrue($result->isNoVerdict());
        self::assertFalse($result->hasState());
        self::assertFalse($result->isQueryError());
        self::assertSame($stage, $result->getStage());
        self::assertSame($httpStatus === null ? 'timeout' : 'http_status', $result->getNoVerdictReason());
        self::assertSame($httpStatus, $result->getHttpStatus());
        self::assertNull($result->getState());
        self::assertNull($result->getStateCode());
        self::assertSame($message, $result->getMessage());
    }

    public function testTheBinaryGetsTheQueryOnStdinAndTheSecretsOnlyInItsEnvironment(): void
    {
        $stdin = $this->file('');
        $env = $this->file('');
        $argv = $this->file('');
        $script = sprintf(
            'cat > %s; env > %s; printf "%%s\n" "$@" > %s; printf \'%%s\' %s',
            escapeshellarg($stdin),
            escapeshellarg($env),
            escapeshellarg($argv),
            escapeshellarg('{"outcome":"no_verdict","stage":"token","reason":"timeout","http_status":null,"message":"x"}')
        );
        $sdk = new SunatSdk($this->fakeBinary($script), 60.0);
        $credentials = new ApiCredentials(self::CLIENT_ID, self::CLIENT_SECRET);

        $sdk->validity('20100066603', '03', 'B001', 7, '2026-10-15', '118.00', $credentials);
        self::assertSame(
            ['ruc' => '20100066603', 'document_type' => '03', 'series' => 'B001', 'number' => 7, 'issue_date' => '2026-10-15', 'total' => '118.00'],
            json_decode((string) file_get_contents($stdin), true)
        );
        self::assertSame("validity\n", file_get_contents($argv));
        $vars = self::envVars((string) file_get_contents($env));
        self::assertSame(self::CLIENT_ID, $vars['SUNAT_API_CLIENT_ID']);
        self::assertSame(self::CLIENT_SECRET, $vars['SUNAT_API_CLIENT_SECRET']);
        self::assertSame('55', $vars['SUNAT_HTTP_TIMEOUT_SECONDS']);
        self::assertArrayNotHasKey('SUNAT_SOL_USER', $vars);
        self::assertArrayNotHasKey('SUNAT_SOL_PASSWORD', $vars);
        self::assertArrayNotHasKey('SUNAT_VALIDITY_ENDPOINT', $vars);
        self::assertArrayNotHasKey('SUNAT_VALIDITY_TOKEN_ENDPOINT', $vars);

        $sdk->validity('20100066603', '01', 'F001', 9, '2026-10-15', '10', $credentials, '20000000001');
        $input = json_decode((string) file_get_contents($stdin), true);
        self::assertSame('20000000001', $input['requester_ruc']);
    }

    public function testCredentialsNeverShowAndCannotBeEmpty(): void
    {
        $credentials = new ApiCredentials(self::CLIENT_ID, self::CLIENT_SECRET);
        $shown = print_r($credentials, true) . var_export($credentials->__debugInfo(), true);
        ob_start();
        var_dump($credentials);
        $shown .= (string) ob_get_clean();
        self::assertStringNotContainsString(self::CLIENT_ID, $shown);
        self::assertStringNotContainsString(self::CLIENT_SECRET, $shown);

        $this->expectException(\InvalidArgumentException::class);
        new ApiCredentials(self::CLIENT_ID, '');
    }

    public function testSecretsEchoedOnStderrAreRedacted(): void
    {
        $script = 'cat >/dev/null; printf \'{"error":"refused %s / %s"}\' "$SUNAT_API_CLIENT_ID" "$SUNAT_API_CLIENT_SECRET" >&2; exit 1';
        $sdk = new SunatSdk($this->fakeBinary($script), 60.0);

        try {
            $sdk->validity('20100066603', '03', 'B001', 7, '2026-10-15', '118', new ApiCredentials(self::CLIENT_ID, self::CLIENT_SECRET));
            self::fail('expected an SDK error');
        } catch (SdkErrorException $e) {
            self::assertSame('refused [redacted] / [redacted]', $e->getMessage());
            self::assertSame('validity', $e->getCommand());
        }
    }

    public function testAProcessTimeoutTooShortForHttpRefusesBeforeRunning(): void
    {
        $sdk = new SunatSdk($this->fakeBinary('exit 99'), 0.3);

        $this->expectException(\InvalidArgumentException::class);
        $this->expectExceptionMessage('validity');
        $sdk->validity('20100066603', '03', 'B001', 7, '2026-10-15', '118', new ApiCredentials('a', 'b'));
    }

    public function testAHungBinaryIsKilledAtTheProcessTimeout(): void
    {
        $sdk = new SunatSdk($this->fakeBinary('sleep 30'), 6.0, 1);
        $started = microtime(true);

        try {
            $sdk->validity('20100066603', '03', 'B001', 7, '2026-10-15', '118', new ApiCredentials('a', 'b'));
            self::fail('expected a timeout');
        } catch (TimeoutException $e) {
            self::assertStringContainsString('validity', $e->getMessage());
            self::assertLessThan(9.0, microtime(true) - $started);
        }
    }

    /**
     * @return array<string, array{string}>
     */
    public static function brokenOutputs(): array
    {
        return [
            'unknown outcome' => ['{"outcome":"token","access_token":"x"}'],
            'no_verdict without stage' => ['{"outcome":"no_verdict","reason":"timeout","http_status":null}'],
            'http_status as text' => ['{"outcome":"no_verdict","stage":"query","reason":"http_status","http_status":"401"}'],
            'observations not strings' => ['{"outcome":"validity","state_code":"1","state":"accepted","observations":[1]}'],
        ];
    }

    /**
     * @dataProvider brokenOutputs
     */
    #[DataProvider('brokenOutputs')]
    public function testOutputOutsideTheContractIsInvalidOutput(string $json): void
    {
        $sdk = new SunatSdk($this->fakeBinary(sprintf('cat >/dev/null; printf \'%%s\' %s', escapeshellarg($json))), 60.0);

        $this->expectException(InvalidOutputException::class);
        $sdk->validity('20100066603', '03', 'B001', 7, '2026-10-15', '118', new ApiCredentials('a', 'b'));
    }

    /** The real binary refuses an input SUNAT would refuse before the token call: no network. */
    public function testTheRealBinaryRefusesABadQueryBeforeAnyCall(): void
    {
        $sdk = new SunatSdk(self::binaryPath(), 60.0);

        try {
            $sdk->validity('20100066603', '09', 'T001', 7, '2026-10-15', '118', new ApiCredentials(self::CLIENT_ID, self::CLIENT_SECRET));
            self::fail('expected an SDK error');
        } catch (SdkErrorException $e) {
            self::assertStringContainsString('document_type', $e->getMessage());
            self::assertSame(1, $e->getExitCode());
            self::assertStringNotContainsString(self::CLIENT_SECRET, $e->getMessage());
        }
    }

    /**
     * @param array<string, mixed> $output
     */
    private function validity(array $output): ValidityResult
    {
        $json = (string) json_encode($output, JSON_UNESCAPED_UNICODE | JSON_UNESCAPED_SLASHES);
        $sdk = new SunatSdk($this->fakeBinary(sprintf('cat >/dev/null; printf \'%%s\' %s', escapeshellarg($json))), 90.0);

        return $sdk->validity('20100066603', '03', 'B001', 7, '2026-10-15', '118', new ApiCredentials(self::CLIENT_ID, self::CLIENT_SECRET));
    }

    /**
     * @return array<string, mixed>
     */
    private static function recording(string $name): array
    {
        $path = self::repositoryPath('crates/sunat-sdk/tests/data/validity/' . $name . '.json');
        $data = json_decode((string) file_get_contents($path), true);
        self::assertIsArray($data, $path);

        return $data;
    }

    /**
     * @return array<string, string>
     */
    private static function envVars(string $dump): array
    {
        $vars = [];
        foreach (explode("\n", $dump) as $line) {
            $parts = explode('=', $line, 2);
            if (count($parts) === 2) {
                $vars[$parts[0]] = $parts[1];
            }
        }

        return $vars;
    }

    private function file(string $content): string
    {
        $path = (string) tempnam(sys_get_temp_dir(), 'sunat-sdk-validity-');
        file_put_contents($path, $content);
        $this->files[] = $path;

        return $path;
    }

    protected function tearDown(): void
    {
        foreach ($this->files as $path) {
            @unlink($path);
        }
        $this->files = [];
        parent::tearDown();
    }
}
