<?php

declare(strict_types=1);

namespace Kembec\SunatSdk;

use Kembec\SunatSdk\Exception\SunatSdkException;
use Kembec\SunatSdk\Process\BinaryRunner;
use Kembec\SunatSdk\Result\BuildResult;
use Kembec\SunatSdk\Result\Cdr;
use Kembec\SunatSdk\Result\SendResult;
use Kembec\SunatSdk\Result\SignResult;
use Kembec\SunatSdk\Result\StatusResult;
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
    public const VERSION = '0.1.0-beta.2';

    public const DEFAULT_TIMEOUT_SECONDS = 120.0;

    /** HTTP wait of `send` and `status` when none is given and the process timeout allows it. */
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
                'the process timeout is too short for send and status: it needs at least %d s',
                self::HTTP_TIMEOUT_MARGIN_SECONDS + 1
            ));
        }

        return ['SUNAT_HTTP_TIMEOUT_SECONDS' => (string) $this->httpTimeoutSeconds];
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
