<?php

declare(strict_types=1);

namespace Kembec\SunatSdk\Result;

use Kembec\SunatSdk\Exception\InvalidOutputException;

/**
 * Output of `status` (getStatusCdr): the status code and message, with the CDR when attached; a fault;
 * or no verdict, when SUNAT answered an HTTP status instead of a response or the wait ran out. No verdict
 * says nothing about the voucher: ask again, never send it again because of it.
 */
final class StatusResult
{
    /** @var string|null */
    private $statusCode;

    /** @var string|null */
    private $statusMessage;

    /** @var Cdr|null */
    private $cdr;

    /** @var string|null */
    private $cdrZipBase64;

    /** @var Fault|null */
    private $fault;

    /** @var string|null `http_status` or `timeout` */
    private $noVerdictReason;

    /** @var int|null */
    private $httpStatus;

    private function __construct(
        ?string $statusCode,
        ?string $statusMessage,
        ?Cdr $cdr,
        ?string $cdrZipBase64,
        ?Fault $fault,
        ?string $noVerdictReason = null,
        ?int $httpStatus = null
    ) {
        $this->statusCode = $statusCode;
        $this->statusMessage = $statusMessage;
        $this->cdr = $cdr;
        $this->cdrZipBase64 = $cdrZipBase64;
        $this->fault = $fault;
        $this->noVerdictReason = $noVerdictReason;
        $this->httpStatus = $httpStatus;
    }

    /**
     * @param array<string, mixed> $data
     */
    public static function fromArray(array $data): self
    {
        $outcome = Fields::string($data, 'outcome');
        if ($outcome === 'fault') {
            return new self(null, null, null, null, Fault::fromArray($data));
        }
        if ($outcome === 'no_verdict') {
            $httpStatus = $data['http_status'] ?? null;
            if ($httpStatus !== null && !is_int($httpStatus)) {
                throw new InvalidOutputException('sunat-sdk status returned a non-integer `http_status`');
            }

            return new self(null, null, null, null, null, Fields::string($data, 'reason'), $httpStatus);
        }
        if ($outcome !== 'status') {
            throw new InvalidOutputException(sprintf('sunat-sdk status returned an unknown outcome `%s`', $outcome));
        }
        $cdr = Fields::optionalObject($data, 'cdr');

        return new self(
            Fields::string($data, 'status_code'),
            Fields::optionalString($data, 'status_message'),
            $cdr === null ? null : Cdr::fromArray($cdr),
            Fields::optionalString($data, 'cdr_zip_base64'),
            null
        );
    }

    public function isFault(): bool
    {
        return $this->fault !== null;
    }

    /** SUNAT gave no answer (an HTTP status or a timeout): ask again later with the same data. */
    public function isNoVerdict(): bool
    {
        return $this->noVerdictReason !== null;
    }

    /** `http_status` or `timeout` when there is no verdict, null otherwise. */
    public function getNoVerdictReason(): ?string
    {
        return $this->noVerdictReason;
    }

    /** The HTTP status SUNAT answered instead of a response, when that is why there is no verdict. */
    public function getHttpStatus(): ?int
    {
        return $this->httpStatus;
    }

    public function getStatusCode(): ?string
    {
        return $this->statusCode;
    }

    public function getStatusMessage(): ?string
    {
        return $this->statusMessage;
    }

    public function getCdr(): ?Cdr
    {
        return $this->cdr;
    }

    public function getCdrZipBase64(): ?string
    {
        return $this->cdrZipBase64;
    }

    public function getFault(): ?Fault
    {
        return $this->fault;
    }
}
