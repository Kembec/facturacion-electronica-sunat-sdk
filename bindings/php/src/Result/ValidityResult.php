<?php

declare(strict_types=1);

namespace Kembec\SunatSdk\Result;

use Kembec\SunatSdk\Exception\InvalidOutputException;

/**
 * Output of `validity` (Consulta Integrada): SUNAT's `estadoCp` for one voucher; an error about the
 * query itself; or no verdict, when the token or the query call got no answer (401 included).
 *
 * None of them is a reason to issue the operation with another number: `not_reported` does not prove
 * the voucher never arrived, and an error or no verdict says nothing about it (docs/cli.md).
 */
final class ValidityResult
{
    public const NOT_REPORTED = 'not_reported';
    public const ACCEPTED = 'accepted';
    public const VOIDED = 'voided';
    public const PRINTED_AUTHORIZED = 'printed_authorized';
    public const PRINTED_NOT_AUTHORIZED = 'printed_not_authorized';

    /** @var string `validity`, `query_error` or `no_verdict` */
    private $outcome;

    /** @var array<string, mixed> */
    private $data;

    /**
     * @param array<string, mixed> $data
     */
    private function __construct(string $outcome, array $data)
    {
        $this->outcome = $outcome;
        $this->data = $data;
    }

    /**
     * @param array<string, mixed> $data
     */
    public static function fromArray(array $data): self
    {
        $outcome = Fields::string($data, 'outcome');
        if (!in_array($outcome, ['validity', 'query_error', 'no_verdict'], true)) {
            throw new InvalidOutputException(sprintf('sunat-sdk validity returned an unknown outcome `%s`', $outcome));
        }
        foreach (['state_code', 'state', 'taxpayer_state_code', 'domicile_condition_code', 'error_code', 'stage', 'reason'] as $key) {
            Fields::optionalString($data, $key);
        }
        $httpStatus = $data['http_status'] ?? null;
        if ($httpStatus !== null && !is_int($httpStatus)) {
            throw new InvalidOutputException('sunat-sdk validity returned a non-integer `http_status`');
        }
        $observations = $data['observations'] ?? [];
        if (!is_array($observations) || array_filter($observations, 'is_string') !== $observations) {
            throw new InvalidOutputException('sunat-sdk validity returned `observations` that are not strings');
        }
        if ($outcome === 'no_verdict') {
            Fields::string($data, 'stage');
            Fields::string($data, 'reason');
        }

        return new self($outcome, $data);
    }

    /** SUNAT answered with the voucher's state. */
    public function hasState(): bool
    {
        return $this->outcome === 'validity';
    }

    /** SUNAT refused the query itself (malformed or out of its rules): fix the input; nothing about the voucher. */
    public function isQueryError(): bool
    {
        return $this->outcome === 'query_error';
    }

    /** No answer from the token or the query call: ask again later with the same data. */
    public function isNoVerdict(): bool
    {
        return $this->outcome === 'no_verdict';
    }

    /** One of the class constants, or null for a code outside the manual or no state. */
    public function getState(): ?string
    {
        return $this->optional('state');
    }

    /** `estadoCp` as SUNAT sent it (`0` to `4`). */
    public function getStateCode(): ?string
    {
        return $this->optional('state_code');
    }

    /** `estadoRuc` (`00` active, ...). */
    public function getTaxpayerStateCode(): ?string
    {
        return $this->optional('taxpayer_state_code');
    }

    /** `condDomiRuc` (`00` habido, ...). */
    public function getDomicileConditionCode(): ?string
    {
        return $this->optional('domicile_condition_code');
    }

    /**
     * Where the data sent differs from SUNAT's, as SUNAT words it.
     *
     * @return string[]
     */
    public function getObservations(): array
    {
        return $this->data['observations'] ?? [];
    }

    /** `errorCode` of a query error. */
    public function getErrorCode(): ?string
    {
        return $this->optional('error_code');
    }

    /** `token` or `query`: which call got no answer. */
    public function getStage(): ?string
    {
        return $this->optional('stage');
    }

    /** `http_status` or `timeout` when there is no verdict. */
    public function getNoVerdictReason(): ?string
    {
        return $this->isNoVerdict() ? $this->optional('reason') : null;
    }

    public function getHttpStatus(): ?int
    {
        return $this->data['http_status'] ?? null;
    }

    public function getMessage(): ?string
    {
        return $this->optional('message');
    }

    private function optional(string $key): ?string
    {
        $value = $this->data[$key] ?? null;

        return is_string($value) ? $value : null;
    }
}
