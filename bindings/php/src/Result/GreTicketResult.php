<?php

declare(strict_types=1);

namespace Kembec\SunatSdk\Result;

use Kembec\SunatSdk\Exception\InvalidOutputException;

/**
 * Output of `gre-ticket`: the state of a sent guide, SUNAT's refusal of the query, or no verdict. Every
 * one carries the ticket. In process (98) means ask again with the same ticket, never send again.
 * SUNAT has no test environment for GRE.
 */
final class GreTicketResult
{
    public const PROCESSED = 'processed';
    public const IN_PROCESS = 'in_process';
    public const PROCESSED_WITH_ERRORS = 'processed_with_errors';

    /** @var array<string, mixed> */
    private $data;

    /** @var Cdr|null */
    private $cdr;

    /** @var GreApiError|null */
    private $apiError;

    /** @var GreNoVerdict|null */
    private $noVerdict;

    /**
     * @param array<string, mixed> $data
     */
    private function __construct(array $data, ?Cdr $cdr, ?GreApiError $apiError, ?GreNoVerdict $noVerdict)
    {
        $this->data = $data;
        $this->cdr = $cdr;
        $this->apiError = $apiError;
        $this->noVerdict = $noVerdict;
    }

    /**
     * @param array<string, mixed> $data
     */
    public static function fromArray(array $data): self
    {
        $outcome = Fields::string($data, 'outcome');
        Fields::string($data, 'ticket');
        if ($outcome === 'api_error') {
            return new self($data, null, GreApiError::fromArray($data), null);
        }
        if ($outcome === 'no_verdict') {
            return new self($data, null, null, GreNoVerdict::fromArray($data));
        }
        if ($outcome !== 'ticket_status') {
            throw new InvalidOutputException(sprintf('sunat-sdk gre-ticket returned an unknown outcome `%s`', $outcome));
        }
        Fields::string($data, 'status_code');
        Fields::optionalString($data, 'state');
        Fields::optionalString($data, 'cdr_zip_base64');
        if (!is_bool($data['cdr_generated'] ?? null)) {
            throw new InvalidOutputException('sunat-sdk gre-ticket returned no boolean `cdr_generated`');
        }
        $error = Fields::optionalObject($data, 'error');
        if ($error !== null) {
            Fields::optionalString($error, 'code');
            Fields::optionalString($error, 'description');
            Fields::optionalString($error, 'class');
        }
        $cdr = Fields::optionalObject($data, 'cdr');

        return new self($data, $cdr === null ? null : Cdr::fromArray($cdr), null, null);
    }

    /** The ticket asked, whatever the outcome. */
    public function getTicket(): string
    {
        return (string) $this->data['ticket'];
    }

    /** SUNAT answered the query with the guide's state. */
    public function hasStatus(): bool
    {
        return $this->apiError === null && $this->noVerdict === null;
    }

    /** `codRespuesta` as SUNAT sent it (`0`, `98`, `99`). */
    public function getStatusCode(): ?string
    {
        return $this->optional('status_code');
    }

    /** One of the class constants, or null for another code or no status. */
    public function getState(): ?string
    {
        return $this->optional('state');
    }

    /** 98: SUNAT is still processing the guide. Ask again later with the same ticket; never send again. */
    public function isInProcess(): bool
    {
        return $this->getState() === self::IN_PROCESS;
    }

    public function isCdrGenerated(): bool
    {
        return ($this->data['cdr_generated'] ?? false) === true;
    }

    /**
     * `numError` and `desError` of a guide processed with errors, with the class of the code.
     *
     * @return array{code: ?string, description: string, class: ?string}|null
     */
    public function getError(): ?array
    {
        $error = $this->data['error'] ?? null;
        if (!is_array($error)) {
            return null;
        }

        return [
            'code' => is_string($error['code'] ?? null) ? $error['code'] : null,
            'description' => is_string($error['description'] ?? null) ? $error['description'] : '',
            'class' => is_string($error['class'] ?? null) ? $error['class'] : null,
        ];
    }

    /** The CDR SUNAT attached, read like cdr(). */
    public function getCdr(): ?Cdr
    {
        return $this->cdr;
    }

    public function getCdrZipBase64(): ?string
    {
        return $this->optional('cdr_zip_base64');
    }

    public function isApiError(): bool
    {
        return $this->apiError !== null;
    }

    public function getApiError(): ?GreApiError
    {
        return $this->apiError;
    }

    public function isNoVerdict(): bool
    {
        return $this->noVerdict !== null;
    }

    public function getNoVerdict(): ?GreNoVerdict
    {
        return $this->noVerdict;
    }

    private function optional(string $key): ?string
    {
        $value = $this->data[$key] ?? null;

        return is_string($value) ? $value : null;
    }
}
