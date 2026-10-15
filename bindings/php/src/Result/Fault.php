<?php

declare(strict_types=1);

namespace Kembec\SunatSdk\Result;

/** A SOAP fault answered by SUNAT: its code without prefix, message and class. */
final class Fault
{
    /** @var string */
    private $code;

    /** @var string */
    private $message;

    /** @var string|null */
    private $class;

    public function __construct(string $code, string $message, ?string $class)
    {
        $this->code = $code;
        $this->message = $message;
        $this->class = $class;
    }

    /**
     * @param array<string, mixed> $data
     */
    public static function fromArray(array $data): self
    {
        return new self(Fields::string($data, 'code'), Fields::optionalString($data, 'message') ?? '', Fields::optionalString($data, 'class'));
    }

    public function getCode(): string
    {
        return $this->code;
    }

    public function getMessage(): string
    {
        return $this->message;
    }

    public function getClass(): ?string
    {
        return $this->class;
    }
}
