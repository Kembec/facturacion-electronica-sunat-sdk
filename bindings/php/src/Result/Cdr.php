<?php

declare(strict_types=1);

namespace Kembec\SunatSdk\Result;

use Kembec\SunatSdk\Exception\InvalidOutputException;

/** The fields of a CDR (constancia de recepción) and the class of its response code. */
final class Cdr
{
    /** @var string */
    private $referenceId;

    /** @var string */
    private $responseCode;

    /** @var string */
    private $description;

    /** @var string[] */
    private $notes;

    /** @var string|null */
    private $class;

    /**
     * @param string[] $notes
     * @param string|null $class as the binary classifies the response code (`accepted`, `rejected`...); null if unknown
     */
    public function __construct(string $referenceId, string $responseCode, string $description, array $notes, ?string $class)
    {
        $this->referenceId = $referenceId;
        $this->responseCode = $responseCode;
        $this->description = $description;
        $this->notes = $notes;
        $this->class = $class;
    }

    /**
     * @param array<string, mixed> $data
     */
    public static function fromArray(array $data): self
    {
        $notes = $data['notes'] ?? [];
        if (!is_array($notes)) {
            throw new InvalidOutputException('sunat-sdk output field `notes` is not a list');
        }

        return new self(
            Fields::optionalString($data, 'reference_id') ?? '',
            Fields::string($data, 'response_code'),
            Fields::optionalString($data, 'description') ?? '',
            array_map('strval', array_values($notes)),
            Fields::optionalString($data, 'class')
        );
    }

    public function getReferenceId(): string
    {
        return $this->referenceId;
    }

    public function getResponseCode(): string
    {
        return $this->responseCode;
    }

    public function getDescription(): string
    {
        return $this->description;
    }

    /**
     * @return string[]
     */
    public function getNotes(): array
    {
        return $this->notes;
    }

    public function getClass(): ?string
    {
        return $this->class;
    }

    public function isAccepted(): bool
    {
        return $this->class === 'accepted';
    }
}
