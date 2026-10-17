<?php

declare(strict_types=1);

namespace Kembec\SunatSdk\Tests;

use Kembec\SunatSdk\Exception\InvalidOutputException;
use Kembec\SunatSdk\Exception\SdkErrorException;
use Kembec\SunatSdk\Process\BinaryRunner;
use Kembec\SunatSdk\Result\SummaryResult;
use Kembec\SunatSdk\Result\TicketResult;
use Kembec\SunatSdk\SolCredentials;
use Kembec\SunatSdk\SunatSdk;
use PHPUnit\Framework\Attributes\DataProvider;

/**
 * summary() and ticket() (sendSummary and getStatus of billService) from PHP. Each outcome is replayed by
 * a fake binary printing what the core makes of one SOAP answer (crates/sunat-sdk/src/exchange.rs, tested
 * in exchange_tests.rs); the CDR of a processed ticket is read by the real binary. No network.
 */
final class SummaryTicketTest extends BinaryTestCase
{
    private const SOL_USER = '20000000001MODDATOS';
    private const SOL_PASSWORD = 'sol-password-not-real';
    private const ENDPOINT = 'https://sunat.test/billService';
    private const NAME = '20000000001-RC-20261016-1';
    private const TICKET = '1760620000123';

    private const SUMMARY_CDR = '<ar:ApplicationResponse xmlns:ar="urn:oasis:names:specification:ubl:schema:xsd:ApplicationResponse-2"'
        . ' xmlns:cac="urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2"'
        . ' xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2">'
        . '<cac:DocumentResponse><cac:Response><cbc:ReferenceID>RC-20261016-1</cbc:ReferenceID>'
        . '<cbc:ResponseCode>0</cbc:ResponseCode><cbc:Description>El Resumen diario RC-20261016-1, ha sido aceptado</cbc:Description>'
        . '</cac:Response></cac:DocumentResponse></ar:ApplicationResponse>';

    /** @var string[] */
    private $files = [];

    public function testSummaryGivesTheTicketToStoreFirst(): void
    {
        $sent = $this->replaying(['outcome' => 'ticket', 'ticket' => self::TICKET])->summary(self::NAME, '<SummaryDocuments/>', $this->credentials(), self::ENDPOINT);

        self::assertInstanceOf(SummaryResult::class, $sent);
        self::assertTrue($sent->hasTicket());
        self::assertSame(self::TICKET, $sent->getTicket());
        self::assertFalse($sent->isFault());
        self::assertNull($sent->getFault());
    }

    public function testASummaryRefusedIsAFaultWithoutTicket(): void
    {
        $sent = $this->replaying([
            'outcome' => 'fault',
            'code' => '2223',
            'message' => 'El archivo ya fue presentado anteriormente',
            'class' => 'rejected',
        ])->summary(self::NAME, '<SummaryDocuments/>', $this->credentials(), self::ENDPOINT);

        self::assertFalse($sent->hasTicket());
        self::assertNull($sent->getTicket());
        self::assertTrue($sent->isFault());
        self::assertSame('2223', $sent->getFault()->getCode());
        self::assertSame('rejected', $sent->getFault()->getClass());
    }

    /** A summary without answer may have arrived: an error, never a result to resend with another number. */
    public function testASummaryWithoutAnswerIsAnError(): void
    {
        $sdk = $this->replayingError('SUNAT answered HTTP 503');
        try {
            $sdk->summary(self::NAME, '<SummaryDocuments/>', $this->credentials(), self::ENDPOINT);
            self::fail('expected an SDK error');
        } catch (SdkErrorException $e) {
            self::assertSame('SUNAT answered HTTP 503', $e->getMessage());
            self::assertSame('summary', $e->getCommand());
        }

        $late = $this->replayingError('timed out after 55 s waiting for SUNAT; the outcome is unknown');
        $this->expectException(SdkErrorException::class);
        $late->summary(self::NAME, '<SummaryDocuments/>', $this->credentials(), self::ENDPOINT);
    }

    /**
     * @return array<string, array{string, ?string}>
     */
    public static function ticketStates(): array
    {
        return [
            '0 with its CDR' => ['0', TicketResult::PROCESSED],
            '98 in process' => ['98', TicketResult::IN_PROCESS],
            '99 with errors' => ['99', TicketResult::PROCESSED_WITH_ERRORS],
            'another code' => ['0127', null],
        ];
    }

    /**
     * @dataProvider ticketStates
     */
    #[DataProvider('ticketStates')]
    public function testEachTicketState(string $code, ?string $state): void
    {
        $output = ['outcome' => 'ticket_status', 'status_code' => $code, 'state' => $state];
        if ($state === TicketResult::PROCESSED) {
            // The CDR as the real binary reads it, so the replay carries the core's reading.
            $runner = new BinaryRunner(self::binaryPath(), 60.0);
            $zip = $runner->run('zip', (string) json_encode(['name' => self::NAME, 'xml' => self::SUMMARY_CDR]));
            $output['cdr'] = $runner->run('cdr', (string) json_encode(['cdr_base64' => $zip['zip_base64']]));
            $output['cdr_zip_base64'] = $zip['zip_base64'];
        }

        $result = $this->replaying($output)->ticket(self::TICKET, $this->credentials(), self::ENDPOINT);

        self::assertTrue($result->hasStatus());
        self::assertSame(self::TICKET, $result->getTicket());
        self::assertSame($code, $result->getStatusCode());
        self::assertSame($state, $result->getState());
        self::assertSame($state === TicketResult::IN_PROCESS, $result->isInProcess());
        self::assertFalse($result->isNoVerdict());
        self::assertFalse($result->isFault());
        if ($state === TicketResult::PROCESSED) {
            self::assertSame('RC-20261016-1', $result->getCdr()->getReferenceId());
            self::assertTrue($result->getCdr()->isAccepted());
            self::assertStringStartsWith('UEsDB', (string) $result->getCdrZipBase64());
        } else {
            self::assertNull($result->getCdr());
            self::assertNull($result->getCdrZipBase64());
        }
    }

    /**
     * @return array<string, array{?int, string}>
     */
    public static function silences(): array
    {
        return [
            'HTTP 401' => [401, '{"outcome":"no_verdict","reason":"http_status","http_status":401,"message":"SUNAT answered HTTP 401","ticket":"1760620000123"}'],
            'timeout' => [null, '{"outcome":"no_verdict","reason":"timeout","http_status":null,"message":"timed out after 55 s waiting for SUNAT; the outcome is unknown","ticket":"1760620000123"}'],
        ];
    }

    /**
     * @dataProvider silences
     */
    #[DataProvider('silences')]
    public function testATicketWithoutAnswerIsNoVerdictAndKeepsTheTicket(?int $httpStatus, string $json): void
    {
        $result = $this->printing($json)->ticket(self::TICKET, $this->credentials(), self::ENDPOINT);

        self::assertTrue($result->isNoVerdict());
        self::assertFalse($result->hasStatus());
        self::assertSame($httpStatus === null ? 'timeout' : 'http_status', $result->getNoVerdictReason());
        self::assertSame($httpStatus, $result->getHttpStatus());
        self::assertStringStartsWith($httpStatus === null ? 'timed out' : 'SUNAT answered', (string) $result->getMessage());
        self::assertSame(self::TICKET, $result->getTicket());
        self::assertNull($result->getState());
    }

    public function testATicketFaultIsAResult(): void
    {
        $result = $this->replaying([
            'outcome' => 'fault',
            'code' => '0127',
            'message' => 'El ticket no existe',
            'class' => 'exception',
        ])->ticket(self::TICKET, $this->credentials(), self::ENDPOINT);

        self::assertTrue($result->isFault());
        self::assertSame('0127', $result->getFault()->getCode());
        self::assertSame(self::TICKET, $result->getTicket());
        self::assertFalse($result->hasStatus());
    }

    public function testTheBinaryGetsTheInputOnStdinAndSecretsAndEndpointOnlyInItsEnvironment(): void
    {
        $stdin = $this->file();
        $env = $this->file();
        $argv = $this->file();
        $script = sprintf(
            'cat > %s; env > %s; printf "%%s\n" "$@" > %s; printf \'%%s\' %s',
            escapeshellarg($stdin),
            escapeshellarg($env),
            escapeshellarg($argv),
            escapeshellarg('{"outcome":"ticket","ticket":"1760620000123"}')
        );
        $sdk = new SunatSdk($this->fakeBinary($script), 60.0);

        $sdk->summary(self::NAME, '<SummaryDocuments/>', $this->credentials(), self::ENDPOINT);
        self::assertSame(['name' => self::NAME, 'xml' => '<SummaryDocuments/>'], json_decode((string) file_get_contents($stdin), true));
        self::assertSame("summary\n", file_get_contents($argv));
        $vars = self::envVars((string) file_get_contents($env));
        self::assertSame(self::SOL_USER, $vars['SUNAT_SOL_USER']);
        self::assertSame(self::SOL_PASSWORD, $vars['SUNAT_SOL_PASSWORD']);
        self::assertSame(self::ENDPOINT, $vars['SUNAT_ENDPOINT']);
        self::assertSame('55', $vars['SUNAT_HTTP_TIMEOUT_SECONDS']);
        self::assertStringNotContainsString(self::SOL_PASSWORD, (string) file_get_contents($stdin));

        $sdk = new SunatSdk($this->fakeBinary(str_replace('{"outcome":"ticket","ticket":"1760620000123"}', '{"outcome":"ticket_status","status_code":"98","state":"in_process"}', $script)), 60.0);
        $sdk->ticket(self::TICKET, $this->credentials(), self::ENDPOINT);
        self::assertSame(['ticket' => self::TICKET], json_decode((string) file_get_contents($stdin), true));
        self::assertSame("ticket\n", file_get_contents($argv));
        self::assertSame(self::ENDPOINT, self::envVars((string) file_get_contents($env))['SUNAT_ENDPOINT']);
    }

    /**
     * @return array<string, array{string, string}>
     */
    public static function brokenOutputs(): array
    {
        return [
            'summary: unknown outcome' => ['summary', '{"outcome":"cdr","cdr":{"response_code":"0"}}'],
            'summary: ticket without ticket' => ['summary', '{"outcome":"ticket"}'],
            'ticket: without status_code' => ['ticket', '{"outcome":"ticket_status","state":"in_process"}'],
            'ticket: no_verdict without reason' => ['ticket', '{"outcome":"no_verdict","http_status":null,"ticket":"1760620000123"}'],
            'ticket: http_status as text' => ['ticket', '{"outcome":"no_verdict","reason":"http_status","http_status":"401","ticket":"1760620000123"}'],
            'ticket: another ticket' => ['ticket', '{"outcome":"no_verdict","reason":"timeout","http_status":null,"ticket":"999"}'],
        ];
    }

    /**
     * @dataProvider brokenOutputs
     */
    #[DataProvider('brokenOutputs')]
    public function testOutputOutsideTheContractIsInvalidOutput(string $call, string $json): void
    {
        $sdk = $this->printing($json);

        $this->expectException(InvalidOutputException::class);
        if ($call === 'summary') {
            $sdk->summary(self::NAME, '<x/>', $this->credentials(), self::ENDPOINT);
        } else {
            $sdk->ticket(self::TICKET, $this->credentials(), self::ENDPOINT);
        }
    }

    /** The real binary refuses a plain-http endpoint before any call: no network. */
    public function testTheRealBinaryRefusesAPlainHttpEndpoint(): void
    {
        $sdk = new SunatSdk(self::binaryPath(), 60.0);
        foreach (['summary', 'ticket'] as $call) {
            try {
                if ($call === 'summary') {
                    $sdk->summary(self::NAME, '<x/>', $this->credentials(), 'http://plain.example/billService');
                } else {
                    $sdk->ticket(self::TICKET, $this->credentials(), 'http://plain.example/billService');
                }
                self::fail('expected an SDK error: ' . $call);
            } catch (SdkErrorException $e) {
                self::assertSame(1, $e->getExitCode());
                self::assertSame($call, $e->getCommand());
                self::assertStringNotContainsString(self::SOL_PASSWORD, $e->getMessage());
            }
        }
    }

    private function credentials(): SolCredentials
    {
        return new SolCredentials(self::SOL_USER, self::SOL_PASSWORD);
    }

    /**
     * @param array<string, mixed> $output
     */
    private function replaying(array $output): SunatSdk
    {
        return $this->printing((string) json_encode($output, JSON_UNESCAPED_UNICODE | JSON_UNESCAPED_SLASHES));
    }

    private function printing(string $json): SunatSdk
    {
        return new SunatSdk($this->fakeBinary(sprintf('cat >/dev/null; printf \'%%s\' %s', escapeshellarg($json))), 90.0);
    }

    /** The binary's own error for a send without answer: exit 1 and `{"error"}` on stderr. */
    private function replayingError(string $message): SunatSdk
    {
        $json = (string) json_encode(['error' => $message]);

        return new SunatSdk($this->fakeBinary(sprintf('cat >/dev/null; printf \'%%s\' %s >&2; exit 1', escapeshellarg($json))), 90.0);
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

    private function file(): string
    {
        $path = (string) tempnam(sys_get_temp_dir(), 'sunat-sdk-summary-');
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
