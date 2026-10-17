<?php

declare(strict_types=1);

namespace Kembec\SunatSdk\Result;

use Kembec\SunatSdk\Exception\InvalidOutputException;

/** Output of `send`: either the CDR SUNAT returned, or the SOAP fault it answered with. */
final class SendResult
{
    /** @var Cdr|null */
    private $cdr;

    /** @var string|null */
    private $cdrZipBase64;

    /** @var Fault|null */
    private $fault;

    /** @var Diagnosis[] */
    private $diagnosis = [];

    private function __construct(?Cdr $cdr, ?string $cdrZipBase64, ?Fault $fault)
    {
        $this->cdr = $cdr;
        $this->cdrZipBase64 = $cdrZipBase64;
        $this->fault = $fault;
    }

    /**
     * @param array<string, mixed> $data
     */
    public static function fromArray(array $data): self
    {
        $result = self::read($data);
        $result->diagnosis = Diagnosis::listFrom($data);

        return $result;
    }

    /**
     * @param array<string, mixed> $data
     */
    private static function read(array $data): self
    {
        $outcome = Fields::string($data, 'outcome');
        if ($outcome === 'fault') {
            return new self(null, null, Fault::fromArray($data));
        }
        $cdr = Fields::optionalObject($data, 'cdr');
        if ($outcome !== 'cdr' || $cdr === null) {
            throw new InvalidOutputException(sprintf('sunat-sdk send returned an unknown outcome `%s`', $outcome));
        }

        return new self(Cdr::fromArray($cdr), Fields::optionalString($data, 'cdr_zip_base64'), null);
    }

    public function isFault(): bool
    {
        return $this->fault !== null;
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

    /**
     * The input field each code in the answer points at, one Diagnosis per code: the fault's, a CDR's
     * non-zero response code and each note that starts with a code. Empty when the binary sends none
     * (no verdict, or an answer without codes).
     *
     * @return Diagnosis[]
     */
    public function getDiagnosis(): array
    {
        return $this->diagnosis;
    }
}
