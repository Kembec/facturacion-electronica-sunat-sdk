<?php

declare(strict_types=1);

namespace Kembec\SunatSdk\Result;

/** Output of `zip`: the `.zip` file name and its content. */
final class ZipResult
{
    /** @var string */
    private $fileName;

    /** @var string */
    private $zipBase64;

    public function __construct(string $fileName, string $zipBase64)
    {
        $this->fileName = $fileName;
        $this->zipBase64 = $zipBase64;
    }

    /**
     * @param array<string, mixed> $data
     */
    public static function fromArray(array $data): self
    {
        return new self(Fields::string($data, 'file_name'), Fields::string($data, 'zip_base64'));
    }

    public function getFileName(): string
    {
        return $this->fileName;
    }

    public function getZipBase64(): string
    {
        return $this->zipBase64;
    }

    public function getZipBytes(): string
    {
        return (string) base64_decode($this->zipBase64, true);
    }
}
