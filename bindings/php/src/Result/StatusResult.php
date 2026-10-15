<?php

declare(strict_types=1);

namespace Kembec\SunatSdk\Result;

use Kembec\SunatSdk\Exception\InvalidOutputException;

/** Output of `status` (getStatusCdr): the status code and message, with the CDR when attached, or a fault. */
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

    private function __construct(?string $statusCode, ?string $statusMessage, ?Cdr $cdr, ?string $cdrZipBase64, ?Fault $fault)
    {
        $this->statusCode = $statusCode;
        $this->statusMessage = $statusMessage;
        $this->cdr = $cdr;
        $this->cdrZipBase64 = $cdrZipBase64;
        $this->fault = $fault;
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
