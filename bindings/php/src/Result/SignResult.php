<?php

declare(strict_types=1);

namespace Kembec\SunatSdk\Result;

/** Output of `sign`: the signed XML with its DigestValue and SignatureValue. */
final class SignResult
{
    /** @var string */
    private $xml;

    /** @var string */
    private $digestValue;

    /** @var string */
    private $signatureValue;

    public function __construct(string $xml, string $digestValue, string $signatureValue)
    {
        $this->xml = $xml;
        $this->digestValue = $digestValue;
        $this->signatureValue = $signatureValue;
    }

    /**
     * @param array<string, mixed> $data
     */
    public static function fromArray(array $data): self
    {
        return new self(
            Fields::string($data, 'xml'),
            Fields::string($data, 'digest_value'),
            Fields::string($data, 'signature_value')
        );
    }

    public function getXml(): string
    {
        return $this->xml;
    }

    public function getDigestValue(): string
    {
        return $this->digestValue;
    }

    public function getSignatureValue(): string
    {
        return $this->signatureValue;
    }
}
