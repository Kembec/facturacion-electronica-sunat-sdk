<?php

declare(strict_types=1);

namespace Kembec\SunatSdk\Result;

use Kembec\SunatSdk\Exception\InvalidOutputException;

/**
 * SUNAT answered a GRE call with the error body of its GRE manual (annex I, or 422 with the validation
 * errors). Nothing about the guide was decided by it: fix what the errors say and try again.
 */
final class GreApiError
{
    /** @var string `token`, `send` or `ticket` */
    private $stage;

    /** @var int */
    private $httpStatus;

    /** @var string|null */
    private $generalError;

    /** @var string|null */
    private $code;

    /** @var string */
    private $message;

    /** @var array<int, array{code: ?string, description: string}> */
    private $errors;

    /**
     * @param array<int, array{code: ?string, description: string}> $errors
     */
    private function __construct(string $stage, int $httpStatus, ?string $generalError, ?string $code, string $message, array $errors)
    {
        $this->stage = $stage;
        $this->httpStatus = $httpStatus;
        $this->generalError = $generalError;
        $this->code = $code;
        $this->message = $message;
        $this->errors = $errors;
    }

    /**
     * @param array<string, mixed> $data
     */
    public static function fromArray(array $data): self
    {
        $httpStatus = $data['http_status'] ?? null;
        if (!is_int($httpStatus)) {
            throw new InvalidOutputException('sunat-sdk returned an `api_error` without an integer `http_status`');
        }
        $items = $data['errors'] ?? [];
        if (!is_array($items)) {
            throw new InvalidOutputException('sunat-sdk returned `errors` that are not a list');
        }
        $errors = [];
        foreach ($items as $item) {
            if (!is_array($item)) {
                throw new InvalidOutputException('sunat-sdk returned an `errors` item that is not an object');
            }
            $errors[] = ['code' => Fields::optionalString($item, 'code'), 'description' => Fields::optionalString($item, 'description') ?? ''];
        }

        return new self(
            Fields::string($data, 'stage'),
            $httpStatus,
            Fields::optionalString($data, 'general_error'),
            Fields::optionalString($data, 'code'),
            Fields::optionalString($data, 'message') ?? '',
            $errors
        );
    }

    /** `token`, `send` or `ticket`: the call SUNAT refused. */
    public function getStage(): string
    {
        return $this->stage;
    }

    public function getHttpStatus(): int
    {
        return $this->httpStatus;
    }

    /** The annex I name of the status (`unauthorized`, `unprocessable_entity`...), null outside it. */
    public function getGeneralError(): ?string
    {
        return $this->generalError;
    }

    /** `cod` of the error body. */
    public function getCode(): ?string
    {
        return $this->code;
    }

    /** `msg` of the error body. */
    public function getMessage(): string
    {
        return $this->message;
    }

    /**
     * The validation errors of a 422, as SUNAT words them.
     *
     * @return array<int, array{code: ?string, description: string}>
     */
    public function getErrors(): array
    {
        return $this->errors;
    }
}
