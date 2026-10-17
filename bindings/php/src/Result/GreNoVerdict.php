<?php

declare(strict_types=1);

namespace Kembec\SunatSdk\Result;

use Kembec\SunatSdk\Exception\InvalidOutputException;

/**
 * A GRE call got no answer, or an answer that is not SUNAT's error body: ask again with the same data.
 * A token without answer sent nothing; a ticket without answer says nothing about the guide.
 */
final class GreNoVerdict
{
    /** @var string `token` or `ticket` */
    private $stage;

    /** @var string `http_status` or `timeout` */
    private $reason;

    /** @var int|null */
    private $httpStatus;

    /** @var string */
    private $message;

    private function __construct(string $stage, string $reason, ?int $httpStatus, string $message)
    {
        $this->stage = $stage;
        $this->reason = $reason;
        $this->httpStatus = $httpStatus;
        $this->message = $message;
    }

    /**
     * @param array<string, mixed> $data
     */
    public static function fromArray(array $data): self
    {
        $httpStatus = $data['http_status'] ?? null;
        if ($httpStatus !== null && !is_int($httpStatus)) {
            throw new InvalidOutputException('sunat-sdk returned a non-integer `http_status`');
        }

        return new self(
            Fields::string($data, 'stage'),
            Fields::string($data, 'reason'),
            $httpStatus,
            Fields::optionalString($data, 'message') ?? ''
        );
    }

    /** `token` or `ticket`: which call got no answer. */
    public function getStage(): string
    {
        return $this->stage;
    }

    /** `http_status` or `timeout`. */
    public function getReason(): string
    {
        return $this->reason;
    }

    public function getHttpStatus(): ?int
    {
        return $this->httpStatus;
    }

    public function getMessage(): string
    {
        return $this->message;
    }
}
