<?php

declare(strict_types=1);

namespace Kembec\SunatSdk\Result;

/** One candidate field for a code, from SUNAT's validation rules of 26.08.2026 (docs/cli.md#diagnosis). */
final class DiagnosisField
{
    public const SOURCE = 'fuente';
    public const INFERRED = 'inferido';

    /** @var string|null */
    private $inputPath;

    /** @var string|null */
    private $xpath;

    /** @var string */
    private $certainty;

    public function __construct(?string $inputPath, ?string $xpath, string $certainty)
    {
        $this->inputPath = $inputPath;
        $this->xpath = $xpath;
        $this->certainty = $certainty;
    }

    /**
     * @param array<string, mixed> $data
     */
    public static function fromArray(array $data): self
    {
        return new self(Fields::optionalString($data, 'input_path'), Fields::optionalString($data, 'xpath'), Fields::string($data, 'certainty'));
    }

    /**
     * A field of the input model (`customer.document_type`, `lines[].unit_price`), `sdk` when the SDK
     * writes the node itself, or null when the input model has no such field.
     */
    public function getInputPath(): ?string
    {
        return $this->inputPath;
    }

    /** The node the rule checks; null for rules on the file, the signature or the service. */
    public function getXpath(): ?string
    {
        return $this->xpath;
    }

    /** `fuente` (SOURCE) when the node is the rule's own and one field writes it, `inferido` (INFERRED) otherwise. */
    public function getCertainty(): string
    {
        return $this->certainty;
    }
}
