<?php

declare(strict_types=1);

namespace Kembec\SunatSdk\Tests;

use Kembec\SunatSdk\ApiCredentials;
use Kembec\SunatSdk\Exception\InvalidOutputException;
use Kembec\SunatSdk\Exception\SdkErrorException;
use Kembec\SunatSdk\Process\BinaryRunner;
use Kembec\SunatSdk\Result\GreSendResult;
use Kembec\SunatSdk\Result\GreTicketResult;
use Kembec\SunatSdk\SolCredentials;
use Kembec\SunatSdk\SunatSdk;
use PHPUnit\Framework\Attributes\DataProvider;

/**
 * greSend() and greTicket() (despatch advice 09 by SUNAT's GRE REST API) from PHP. The binary only takes
 * https:// bases and SUNAT has no test environment for GRE, so each outcome is replayed by a fake binary
 * printing what the core makes of one recorded exchange in crates/sunat-sdk/tests/data/gre/ (sources in
 * its README; the mapping itself is tested in crates/sunat-sdk/src/gre_tests.rs). None of them proves
 * acceptance by SUNAT. No network.
 */
final class GreTest extends BinaryTestCase
{
    private const CLIENT_ID = 'c04864ce-f454-48e1-b41d-8730e575b905';
    private const CLIENT_SECRET = 'gre-secret-not-real==';
    private const SOL_USER = '20000000001MODUSER';
    private const SOL_PASSWORD = 'sol-password-not-real';
    private const TOKEN_ENDPOINT = 'https://auth.example.invalid/v1/clientessol';
    private const ENDPOINT = 'https://api.example.invalid/v1';
    private const NAME = '20000000001-09-T001-1';
    private const HASH = 'f3b5d0c1a8e2f4b6d8a0c2e4f6b8d0a2c4e6f8b0d2a4c6e8f0b2d4a6c8e0f2b4';

    /** @var string[] */
    private $files = [];

    public function testSendGivesTheTicketToStoreFirst(): void
    {
        $recorded = self::recording('fork-send-200');
        $sent = $this->fakeSend([
            'outcome' => 'ticket',
            'ticket' => $recorded['numTicket'],
            'received_at' => $recorded['fecRecepcion'],
            'zip_name' => self::NAME . '.zip',
            'hash_zip' => self::HASH,
        ]);
        self::assertSame('a.b.c', $sent->getTicket());
        self::assertSame('2023-01-10T20:10:50', $sent->getReceivedAt());
        self::assertSame(self::NAME . '.zip', $sent->getZipName());
        self::assertSame(self::HASH, $sent->getHashZip());
        self::assertFalse($sent->isApiError());
        self::assertTrue($sent->hasTicket());
        self::assertFalse($sent->isNoVerdict());
    }

    public function testASendRefusedCarriesTheValidationErrors(): void
    {
        $recorded = self::recording('fork-send-422');
        $sent = $this->fakeSend([
            'outcome' => 'api_error',
            'stage' => 'send',
            'http_status' => 422,
            'general_error' => 'unprocessable_entity',
            'code' => $recorded['cod'],
            'message' => $recorded['msg'],
            'errors' => [['code' => $recorded['errors'][0]['cod'], 'description' => $recorded['errors'][0]['msg']]],
        ]);

        self::assertFalse($sent->hasTicket());
        self::assertNull($sent->getTicket());
        $error = $sent->getApiError();
        self::assertNotNull($error);
        self::assertSame('send', $error->getStage());
        self::assertSame(422, $error->getHttpStatus());
        self::assertSame('unprocessable_entity', $error->getGeneralError());
        self::assertSame([['code' => '501', 'description' => 'El valor de codCpe no permitido o no valido']], $error->getErrors());
    }

    public function testARefusedTokenSendsNothing(): void
    {
        $recorded = self::recording('derived-error-401');
        $sent = $this->fakeSend([
            'outcome' => 'api_error',
            'stage' => 'token',
            'http_status' => 401,
            'general_error' => 'unauthorized',
            'code' => $recorded['cod'],
            'message' => $recorded['msg'],
            'errors' => [],
        ]);

        self::assertTrue($sent->isApiError());
        self::assertSame('token', $sent->getApiError()->getStage());
        self::assertSame('401', $sent->getApiError()->getCode());
        self::assertSame([], $sent->getApiError()->getErrors());
    }

    /**
     * Recorded ticket answer → state, CDR or error.
     *
     * @return array<string, array{string, string}>
     */
    public static function tickets(): array
    {
        return [
            '0 with its CDR' => ['fork-ticket-0', GreTicketResult::PROCESSED],
            '98 in process' => ['fork-ticket-98', GreTicketResult::IN_PROCESS],
            '99 with error 1345' => ['fork-ticket-99', GreTicketResult::PROCESSED_WITH_ERRORS],
        ];
    }

    /**
     * @dataProvider tickets
     */
    #[DataProvider('tickets')]
    public function testEachTicketState(string $recording, string $state): void
    {
        $recorded = self::recording($recording);
        $output = [
            'outcome' => 'ticket_status',
            'ticket' => 'a.b.c',
            'status_code' => $recorded['codRespuesta'],
            'state' => $state,
            'cdr_generated' => $recorded['indCdrGenerado'] === '1',
        ];
        if (isset($recorded['error'])) {
            $output['error'] = ['code' => $recorded['error']['numError'], 'description' => $recorded['error']['desError'], 'class' => 'exception'];
        }
        if (isset($recorded['arcCdr'])) {
            // The CDR as the real binary reads it, so the replay carries the core's reading of the recording.
            $runner = new BinaryRunner(self::binaryPath(), 60.0);
            $output['cdr'] = $runner->run('cdr', (string) json_encode(['cdr_base64' => $recorded['arcCdr']]));
            $output['cdr_zip_base64'] = $recorded['arcCdr'];
        }

        $result = $this->fakeTicket($output);

        self::assertTrue($result->hasStatus());
        self::assertSame('a.b.c', $result->getTicket());
        self::assertSame($recorded['codRespuesta'], $result->getStatusCode());
        self::assertSame($state, $result->getState());
        self::assertSame($state === GreTicketResult::IN_PROCESS, $result->isInProcess());
        self::assertSame($state === GreTicketResult::PROCESSED, $result->isCdrGenerated());
        if ($state === GreTicketResult::PROCESSED) {
            self::assertSame('T001-1', $result->getCdr()->getReferenceId());
            self::assertSame('observed', $result->getCdr()->getClass(), 'note 4398 on an accepted guide');
            self::assertTrue($result->getCdr()->isValidDocument());
            self::assertStringStartsWith('UEsDB', (string) $result->getCdrZipBase64());
        } else {
            self::assertNull($result->getCdr());
        }
        $expectedError = $state === GreTicketResult::PROCESSED_WITH_ERRORS
            ? ['code' => '1345', 'description' => 'El RUC no valido', 'class' => 'exception']
            : null;
        self::assertSame($expectedError, $result->getError());
    }

    public function testATicketQueryRefusedAndTheSilencesKeepTheTicket(): void
    {
        $recorded = self::recording('manual-error-422-ticket');
        $refused = $this->fakeTicket([
            'outcome' => 'api_error',
            'stage' => 'ticket',
            'http_status' => 422,
            'general_error' => 'unprocessable_entity',
            'code' => $recorded['cod'],
            'message' => $recorded['msg'],
            'errors' => [['code' => $recorded['errors'][0]['codError'], 'description' => $recorded['errors'][0]['desError']]],
            'ticket' => 'a.b.c',
        ]);
        self::assertSame('a.b.c', $refused->getTicket());
        self::assertSame('166', $refused->getApiError()->getErrors()[0]['code']);
        self::assertFalse($refused->hasStatus());

        $late = $this->fakeTicket([
            'outcome' => 'no_verdict',
            'stage' => 'ticket',
            'reason' => 'timeout',
            'http_status' => null,
            'message' => 'timed out after 55 s waiting for the ticket answer',
            'ticket' => 'a.b.c',
        ]);
        self::assertTrue($late->isNoVerdict());
        self::assertSame('ticket', $late->getNoVerdict()->getStage());
        self::assertSame('timeout', $late->getNoVerdict()->getReason());
        self::assertNull($late->getNoVerdict()->getHttpStatus());
        self::assertSame('a.b.c', $late->getTicket());
        self::assertNull($late->getState());
    }

    public function testTheBinaryGetsTheGuideOnStdinAndSecretsAndEndpointsOnlyInItsEnvironment(): void
    {
        $stdin = $this->file('');
        $env = $this->file('');
        $argv = $this->file('');
        $script = sprintf(
            'cat > %s; env > %s; printf "%%s\n" "$@" > %s; printf \'%%s\' %s',
            escapeshellarg($stdin),
            escapeshellarg($env),
            escapeshellarg($argv),
            escapeshellarg('{"outcome":"no_verdict","stage":"token","reason":"timeout","http_status":null,"message":"x","ticket":"a.b.c"}')
        );
        $sdk = new SunatSdk($this->fakeBinary($script), 60.0);

        $sdk->greSend(self::NAME, '<DespatchAdvice/>', ...array_merge($this->credentials(), self::endpoints()));
        self::assertSame(['name' => self::NAME, 'xml' => '<DespatchAdvice/>'], json_decode((string) file_get_contents($stdin), true));
        self::assertSame("gre-send\n", file_get_contents($argv));
        $vars = self::envVars((string) file_get_contents($env));
        self::assertSame(self::CLIENT_ID, $vars['SUNAT_API_CLIENT_ID']);
        self::assertSame(self::CLIENT_SECRET, $vars['SUNAT_API_CLIENT_SECRET']);
        self::assertSame(self::SOL_USER, $vars['SUNAT_SOL_USER']);
        self::assertSame(self::SOL_PASSWORD, $vars['SUNAT_SOL_PASSWORD']);
        self::assertSame(self::TOKEN_ENDPOINT, $vars['SUNAT_GRE_TOKEN_ENDPOINT']);
        self::assertSame(self::ENDPOINT, $vars['SUNAT_GRE_ENDPOINT']);
        self::assertSame('55', $vars['SUNAT_HTTP_TIMEOUT_SECONDS']);

        $sdk->greTicket('a.b.c', ...array_merge($this->credentials(), self::endpoints()));
        self::assertSame(['ticket' => 'a.b.c'], json_decode((string) file_get_contents($stdin), true));
        self::assertSame("gre-ticket\n", file_get_contents($argv));
    }

    public function testSecretsEchoedOnStderrAreRedacted(): void
    {
        $script = 'cat >/dev/null; printf \'{"error":"refused %s / %s"}\' "$SUNAT_API_CLIENT_SECRET" "$SUNAT_SOL_PASSWORD" >&2; exit 1';
        $sdk = new SunatSdk($this->fakeBinary($script), 60.0);

        try {
            $sdk->greSend(self::NAME, '<DespatchAdvice/>', ...array_merge($this->credentials(), self::endpoints()));
            self::fail('expected an SDK error');
        } catch (SdkErrorException $e) {
            self::assertSame('refused [redacted] / [redacted]', $e->getMessage());
            self::assertSame('gre-send', $e->getCommand());
        }
    }

    /**
     * @return array<string, array{string, string}>
     */
    public static function brokenOutputs(): array
    {
        return [
            'send: unknown outcome' => ['send', '{"outcome":"token","access_token":"x"}'],
            'send: ticket without hash' => ['send', '{"outcome":"ticket","ticket":"a.b.c","zip_name":"x.zip"}'],
            'send: api_error without status' => ['send', '{"outcome":"api_error","stage":"send","errors":[]}'],
            'ticket: without ticket' => ['ticket', '{"outcome":"ticket_status","status_code":"98","state":"in_process","cdr_generated":false}'],
            'ticket: cdr_generated as text' => ['ticket', '{"outcome":"ticket_status","ticket":"a","status_code":"0","state":"processed","cdr_generated":"1"}'],
            'ticket: no_verdict without stage' => ['ticket', '{"outcome":"no_verdict","ticket":"a","reason":"timeout","http_status":null}'],
        ];
    }

    /**
     * @dataProvider brokenOutputs
     */
    #[DataProvider('brokenOutputs')]
    public function testOutputOutsideTheContractIsInvalidOutput(string $call, string $json): void
    {
        $sdk = new SunatSdk($this->fakeBinary(sprintf('cat >/dev/null; printf \'%%s\' %s', escapeshellarg($json))), 60.0);

        $this->expectException(InvalidOutputException::class);
        if ($call === 'send') {
            $sdk->greSend(self::NAME, '<x/>', ...array_merge($this->credentials(), self::endpoints()));
        } else {
            $sdk->greTicket('a', ...array_merge($this->credentials(), self::endpoints()));
        }
    }

    /** The real binary refuses what SUNAT would refuse before the token call: no network. */
    public function testTheRealBinaryRefusesABadGuideOrTicketBeforeAnyCall(): void
    {
        $sdk = new SunatSdk(self::binaryPath(), 60.0);
        $cases = [
            'RUC-09' => function () use ($sdk): void {
                $sdk->greSend('20000000001-31-V001-1', '<x/>', ...array_merge($this->credentials(), self::endpoints()));
            },
            'ticket must be' => function () use ($sdk): void {
                $sdk->greTicket('a/b', ...array_merge($this->credentials(), self::endpoints()));
            },
            'https' => function () use ($sdk): void {
                $sdk->greTicket('a.b.c', ...array_merge($this->credentials(), ['http://auth.example.invalid', self::ENDPOINT]));
            },
        ];
        foreach ($cases as $expected => $call) {
            try {
                $call();
                self::fail('expected an SDK error: ' . $expected);
            } catch (SdkErrorException $e) {
                self::assertStringContainsString($expected, $e->getMessage());
                self::assertSame(1, $e->getExitCode());
                self::assertStringNotContainsString(self::SOL_PASSWORD, $e->getMessage());
            }
        }
    }

    /**
     * @return array{ApiCredentials, SolCredentials}
     */
    private function credentials(): array
    {
        return [new ApiCredentials(self::CLIENT_ID, self::CLIENT_SECRET), new SolCredentials(self::SOL_USER, self::SOL_PASSWORD)];
    }

    /**
     * @return array{string, string}
     */
    private static function endpoints(): array
    {
        return [self::TOKEN_ENDPOINT, self::ENDPOINT];
    }

    /**
     * @param array<string, mixed> $output
     */
    private function fakeSend(array $output): GreSendResult
    {
        return $this->replaying($output)->greSend(self::NAME, '<DespatchAdvice/>', ...array_merge($this->credentials(), self::endpoints()));
    }

    /**
     * @param array<string, mixed> $output
     */
    private function fakeTicket(array $output): GreTicketResult
    {
        return $this->replaying($output)->greTicket('a.b.c', ...array_merge($this->credentials(), self::endpoints()));
    }

    /**
     * @param array<string, mixed> $output
     */
    private function replaying(array $output): SunatSdk
    {
        $json = (string) json_encode($output, JSON_UNESCAPED_UNICODE | JSON_UNESCAPED_SLASHES);

        return new SunatSdk($this->fakeBinary(sprintf('cat >/dev/null; printf \'%%s\' %s', escapeshellarg($json))), 90.0);
    }

    /**
     * @return array<string, mixed>
     */
    private static function recording(string $name): array
    {
        $path = self::repositoryPath('crates/sunat-sdk/tests/data/gre/' . $name . '.json');
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
        $path = (string) tempnam(sys_get_temp_dir(), 'sunat-sdk-gre-');
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
