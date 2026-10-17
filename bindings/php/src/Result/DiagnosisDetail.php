<?php

declare(strict_types=1);

namespace Kembec\SunatSdk\Result;

/**
 * What SUNAT's message names as `(nodo: "<node>" valor: "<value>")`, e.g. `cbc:ID/schemeID` = `1`.
 * The form is observed in beta answers; SUNAT does not document it.
 */
final class DiagnosisDetail
{
    /** @var string */
    private $node;

    /** @var string */
    private $value;

    public function __construct(string $node, string $value)
    {
        $this->node = $node;
        $this->value = $value;
    }

    /**
     * @param array<string, mixed> $data
     */
    public static function fromArray(array $data): self
    {
        return new self(Fields::string($data, 'node'), Fields::string($data, 'value'));
    }

    /** The last element and, if any, its attribute, as SUNAT writes it. */
    public function getNode(): string
    {
        return $this->node;
    }

    /** The value SUNAT read in that node. */
    public function getValue(): string
    {
        return $this->value;
    }
}
