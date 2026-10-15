<?php

declare(strict_types=1);

namespace Kembec\SunatSdk\Result;

/** Output of `build`: the unsigned UBL XML and the SUNAT file name without extension. */
final class BuildResult
{
    /** @var string */
    private $fileName;

    /** @var string */
    private $xml;

    public function __construct(string $fileName, string $xml)
    {
        $this->fileName = $fileName;
        $this->xml = $xml;
    }

    /**
     * @param array<string, mixed> $data
     */
    public static function fromArray(array $data): self
    {
        return new self(Fields::string($data, 'file_name'), Fields::string($data, 'xml'));
    }

    public function getFileName(): string
    {
        return $this->fileName;
    }

    public function getXml(): string
    {
        return $this->xml;
    }
}
