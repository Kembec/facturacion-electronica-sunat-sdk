<?php

declare(strict_types=1);

namespace Kembec\SunatSdk;

use Kembec\SunatSdk\Exception\SunatSdkException;
use Kembec\SunatSdk\Process\BinaryRunner;
use Kembec\SunatSdk\Result\BuildResult;
use Kembec\SunatSdk\Result\Cdr;
use Kembec\SunatSdk\Result\GreSendResult;
use Kembec\SunatSdk\Result\GreTicketResult;
use Kembec\SunatSdk\Result\SendResult;
use Kembec\SunatSdk\Result\SignResult;
use Kembec\SunatSdk\Result\StatusResult;
use Kembec\SunatSdk\Result\SummaryResult;
use Kembec\SunatSdk\Result\TicketResult;
use Kembec\SunatSdk\Result\ValidityResult;
use Kembec\SunatSdk\Result\ZipResult;

/**
 * Client of the `sunat-sdk` binary (contract: docs/cli.md of the SDK repository). Each method is one
 * process: JSON on stdin, a typed result back, a SunatSdkException subclass on any failure.
 *
 * Endpoints, SOL credentials and key paths go through the child's environment, never argv.
 */
final class SunatSdk
{
    /**
     * Equal to the Cargo workspace version (checked in CI); the binary installer downloads the release
     * `v` . VERSION, so the PHP client and the binary it drives come from the same tag.
     */
    public const VERSION = '1.0.0-rc.4';

    public const DEFAULT_TIMEOUT_SECONDS = 120.0;

    /** HTTP wait of `send`, `summary`, `ticket`, `status`, `validity` and GRE when none is given and the process timeout allows it. */
    public const DEFAULT_HTTP_TIMEOUT_SECONDS = 90;

    /**
     * Time the binary needs besides the HTTP wait (start, zip, parse the answer). The HTTP wait plus
     * this margin must fit in the process timeout, so the binary reports SUNAT's silence as its own
     * error instead of being killed with the outcome lost.
     */
    public const HTTP_TIMEOUT_MARGIN_SECONDS = 5;

    /** @var BinaryRunner */
    private $runner;

    /** @var int|null null when the process timeout is too short to call SUNAT at all */
    private $httpTimeoutSeconds;

    /**
     * @param float $timeoutSeconds per call; the process is killed after it
     * @param int|null $httpTimeoutSeconds wait on SUNAT in `send` and `status`, whole seconds; it must
     *     leave HTTP_TIMEOUT_MARGIN_SECONDS inside $timeoutSeconds. Null: 90, or less if
     *     $timeoutSeconds is shorter (floor($timeoutSeconds) - 5)
     */
    public function __construct(
        string $binaryPath,
        float $timeoutSeconds = self::DEFAULT_TIMEOUT_SECONDS,
        ?int $httpTimeoutSeconds = null
    ) {
        $this->runner = new BinaryRunner($binaryPath, $timeoutSeconds);
        $this->httpTimeoutSeconds = self::httpTimeout($timeoutSeconds, $httpTimeoutSeconds);
    }

    /** The HTTP wait `send` and `status` pass to the binary, or null if they cannot run. */
    public function httpTimeoutSeconds(): ?int
    {
        return $this->httpTimeoutSeconds;
    }

    /**
     * Renders the unsigned UBL XML of one voucher, daily summary or void communication (docs/input-model.md).
     *
     * @param array<string, mixed>|object|string $document the document as an array, an object or a JSON string
     * @throws SunatSdkException
     */
    public function build($document): BuildResult
    {
        $json = is_string($document) ? $document : $this->encode($document);

        return BuildResult::fromArray($this->runner->run('build', $json));
    }

    /**
     * Signs XML produced by `build` with XML-DSig (enveloped, RSA-SHA1 as SUNAT requires).
     *
     * @throws SunatSdkException
     */
    public function sign(string $unsignedXml, SigningKey $key, ?string $signatureId = null): SignResult
    {
        $input = ['xml' => $unsignedXml];
        if ($signatureId !== null) {
            $input['signature_id'] = $signatureId;
        }

        return SignResult::fromArray($this->runner->run('sign', $this->encode($input), $key->toEnvironment()));
    }

    /**
     * Zips the XML under `<name>.xml`, as SUNAT expects it.
     *
     * @param string $name the file name without extension (`20100066603-01-F001-1`)
     * @throws SunatSdkException
     */
    public function zip(string $name, string $xml): ZipResult
    {
        return ZipResult::fromArray($this->runner->run('zip', $this->encode(['name' => $name, 'xml' => $xml])));
    }

    /**
     * Zips the signed XML and sends it with sendBill. A SOAP fault is a result, not an exception.
     *
     * @param string $endpoint the sendBill URL (https only)
     * @throws SunatSdkException
     */
    public function send(string $name, string $signedXml, SolCredentials $credentials, string $endpoint): SendResult
    {
        $env = ['SUNAT_ENDPOINT' => $endpoint] + $this->httpEnvironment() + $credentials->toEnvironment();
        $output = $this->runner->run('send', $this->encode(['name' => $name, 'xml' => $signedXml]), $env);

        return SendResult::fromArray($output);
    }

    /**
     * Zips a signed daily summary (RC) or void communication (RA) and sends it with sendSummary. A SOAP
     * fault is a result. No answer throws SdkErrorException: the summary may have arrived, so check the
     * documents with validity() and resend the same file under the same name, never another number.
     *
     * @param string $name `RUC-RC-YYYYMMDD-N` or `RUC-RA-YYYYMMDD-N`
     * @param string $endpoint the billService URL (https only), the same one send() uses
     * @throws SunatSdkException
     */
    public function summary(string $name, string $signedXml, SolCredentials $credentials, string $endpoint): SummaryResult
    {
        $env = ['SUNAT_ENDPOINT' => $endpoint] + $this->httpEnvironment() + $credentials->toEnvironment();
        $output = $this->runner->run('summary', $this->encode(['name' => $name, 'xml' => $signedXml]), $env);

        return SummaryResult::fromArray($output);
    }

    /**
     * Asks getStatus for the outcome of a summary by its ticket. 98 (in process) and no verdict mean ask
     * again with the same ticket; never send the summary again.
     *
     * @param string $endpoint the billService URL (https only), the same one summary() uses
     * @throws SunatSdkException
     */
    public function ticket(string $ticket, SolCredentials $credentials, string $endpoint): TicketResult
    {
        $env = ['SUNAT_ENDPOINT' => $endpoint] + $this->httpEnvironment() + $credentials->toEnvironment();
        $output = $this->runner->run('ticket', $this->encode(['ticket' => $ticket]), $env);

        return TicketResult::fromArray($output, $ticket);
    }

    /**
     * Asks getStatusCdr for one voucher, with its CDR when SUNAT attaches it.
     *
     * @param string $endpoint the getStatusCdr URL (https only)
     * @throws SunatSdkException
     */
    public function status(
        string $ruc,
        string $documentType,
        string $series,
        int $number,
        SolCredentials $credentials,
        string $endpoint
    ): StatusResult {
        $input = ['ruc' => $ruc, 'document_type' => $documentType, 'series' => $series, 'number' => $number];
        $env = ['SUNAT_STATUS_ENDPOINT' => $endpoint] + $this->httpEnvironment() + $credentials->toEnvironment();

        return StatusResult::fromArray($this->runner->run('status', $this->encode($input), $env));
    }

    /**
     * Asks SUNAT's Consulta Integrada whether a voucher (01, 03, 07, 08) exists and in which state: a
     * token, then the query, both inside the HTTP wait. There is no test environment: this is production.
     *
     * @param string $issueDate `YYYY-MM-DD` or the document's ISO 8601 issue date
     * @param string $total the voucher's total (PayableAmount), up to 2 decimals
     * @param string|null $requesterRuc the RUC that asks; defaults to $ruc
     * @throws SunatSdkException
     */
    public function validity(
        string $ruc,
        string $documentType,
        string $series,
        int $number,
        string $issueDate,
        string $total,
        ApiCredentials $credentials,
        ?string $requesterRuc = null
    ): ValidityResult {
        $input = [
            'ruc' => $ruc,
            'document_type' => $documentType,
            'series' => $series,
            'number' => $number,
            'issue_date' => $issueDate,
            'total' => $total,
        ];
        if ($requesterRuc !== null) {
            $input['requester_ruc'] = $requesterRuc;
        }
        $env = $this->httpEnvironment() + $credentials->toEnvironment();

        return ValidityResult::fromArray($this->runner->run('validity', $this->encode($input), $env));
    }

    /**
     * Sends a signed despatch advice (09) by SUNAT's GRE REST API: a token by password, then the zip
     * with its hashZip, both inside the HTTP wait. SUNAT has no test environment for GRE and the
     * endpoints have no default. No answer to the send throws SdkErrorException: the guide may have
     * arrived, so send the same file again, never the operation with another number.
     *
     * @param string $name `RUC-09-SERIE-NUMERO`
     * @param ApiCredentials $api the GRE pair of «Credenciales de API SUNAT»
     * @param SolCredentials $sol the RUC followed by the SOL user, and its password
     * @param string $tokenEndpoint the token base, https only (GRE manual: `https://api-seguridad.sunat.gob.pe/v1/clientessol`)
     * @param string $endpoint the API base, https only (`https://api-cpe.sunat.gob.pe/v1`)
     * @throws SunatSdkException
     */
    public function greSend(
        string $name,
        string $signedXml,
        ApiCredentials $api,
        SolCredentials $sol,
        string $tokenEndpoint,
        string $endpoint
    ): GreSendResult {
        $env = $this->greEnvironment($api, $sol, $tokenEndpoint, $endpoint);

        return GreSendResult::fromArray($this->runner->run('gre-send', $this->encode(['name' => $name, 'xml' => $signedXml]), $env));
    }

    /**
     * Asks for the outcome of a sent guide by its ticket (a token, then the query). 98 is in process:
     * ask again later with the same ticket.
     *
     * @throws SunatSdkException
     */
    public function greTicket(
        string $ticket,
        ApiCredentials $api,
        SolCredentials $sol,
        string $tokenEndpoint,
        string $endpoint
    ): GreTicketResult {
        $env = $this->greEnvironment($api, $sol, $tokenEndpoint, $endpoint);

        return GreTicketResult::fromArray($this->runner->run('gre-ticket', $this->encode(['ticket' => $ticket]), $env));
    }

    /**
     * Reads a CDR (zip or XML) and classifies its response code.
     *
     * @param string $cdr the raw bytes of the CDR zip or XML
     * @throws SunatSdkException
     */
    public function cdr(string $cdr): Cdr
    {
        return Cdr::fromArray($this->runner->run('cdr', $this->encode(['cdr_base64' => base64_encode($cdr)])));
    }

    private static function httpTimeout(float $timeoutSeconds, ?int $httpTimeoutSeconds): ?int
    {
        $fits = (int) floor($timeoutSeconds) - self::HTTP_TIMEOUT_MARGIN_SECONDS;
        if ($httpTimeoutSeconds === null) {
            $derived = min(self::DEFAULT_HTTP_TIMEOUT_SECONDS, $fits);
            return $derived >= 1 ? $derived : null;
        }
        if ($httpTimeoutSeconds < 1 || $httpTimeoutSeconds > 300 || $httpTimeoutSeconds > $fits) {
            throw new \InvalidArgumentException(sprintf(
                'httpTimeoutSeconds must be 1 to 300 and at most the process timeout minus %d s (%.1f s given, so at most %d)',
                self::HTTP_TIMEOUT_MARGIN_SECONDS,
                $timeoutSeconds,
                max(0, min(300, $fits))
            ));
        }

        return $httpTimeoutSeconds;
    }

    /**
     * @return array<string, string>
     */
    private function httpEnvironment(): array
    {
        if ($this->httpTimeoutSeconds === null) {
            throw new \InvalidArgumentException(sprintf(
                'the process timeout is too short for send, summary, ticket, status, validity and GRE: it needs at least %d s',
                self::HTTP_TIMEOUT_MARGIN_SECONDS + 1
            ));
        }

        return ['SUNAT_HTTP_TIMEOUT_SECONDS' => (string) $this->httpTimeoutSeconds];
    }

    /**
     * @return array<string, string>
     */
    private function greEnvironment(ApiCredentials $api, SolCredentials $sol, string $tokenEndpoint, string $endpoint): array
    {
        $endpoints = ['SUNAT_GRE_TOKEN_ENDPOINT' => $tokenEndpoint, 'SUNAT_GRE_ENDPOINT' => $endpoint];

        return $endpoints + $this->httpEnvironment() + $api->toEnvironment() + $sol->toEnvironment();
    }

    /**
     * @param mixed $value
     */
    private function encode($value): string
    {
        $json = json_encode($value, JSON_UNESCAPED_UNICODE | JSON_UNESCAPED_SLASHES | JSON_PRESERVE_ZERO_FRACTION);
        if ($json === false) {
            throw new \InvalidArgumentException('input cannot be encoded as JSON: ' . json_last_error_msg());
        }

        return $json;
    }
}
