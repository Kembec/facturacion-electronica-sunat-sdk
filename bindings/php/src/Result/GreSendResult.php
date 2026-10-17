<?php

declare(strict_types=1);

namespace Kembec\SunatSdk\Result;

use Kembec\SunatSdk\Exception\InvalidOutputException;

/**
 * Output of `gre-send`: the ticket of the guide (store it first: it is the only handle on the
 * outcome), SUNAT's refusal, or no verdict of the token call (nothing was sent). No answer to the send
 * itself is a SdkErrorException, not a result: the guide may have arrived, so the same file is sent
 * again, never the operation with another number. SUNAT has no test environment for GRE.
 */
final class GreSendResult
{
    /** @var array<string, mixed> */
    private $data;

    /** @var GreApiError|null */
    private $apiError;

    /** @var GreNoVerdict|null */
    private $noVerdict;

    /**
     * @param array<string, mixed> $data
     */
    private function __construct(array $data, ?GreApiError $apiError, ?GreNoVerdict $noVerdict)
    {
        $this->data = $data;
        $this->apiError = $apiError;
        $this->noVerdict = $noVerdict;
    }

    /**
     * @param array<string, mixed> $data
     */
    public static function fromArray(array $data): self
    {
        $outcome = Fields::string($data, 'outcome');
        if ($outcome === 'api_error') {
            return new self($data, GreApiError::fromArray($data), null);
        }
        if ($outcome === 'no_verdict') {
            return new self($data, null, GreNoVerdict::fromArray($data));
        }
        if ($outcome !== 'ticket') {
            throw new InvalidOutputException(sprintf('sunat-sdk gre-send returned an unknown outcome `%s`', $outcome));
        }
        Fields::string($data, 'ticket');
        Fields::string($data, 'zip_name');
        Fields::string($data, 'hash_zip');
        Fields::optionalString($data, 'received_at');

        return new self($data, null, null);
    }

    /** SUNAT took the guide and gave a ticket to ask for its outcome with greTicket(). */
    public function hasTicket(): bool
    {
        return $this->apiError === null && $this->noVerdict === null;
    }

    public function getTicket(): ?string
    {
        return $this->optional('ticket');
    }

    /** `fecRecepcion` as SUNAT sent it. */
    public function getReceivedAt(): ?string
    {
        return $this->optional('received_at');
    }

    /** `<name>.zip`, the file sent. */
    public function getZipName(): ?string
    {
        return $this->optional('zip_name');
    }

    /** SHA-256 of the zip sent, lowercase hex (`hashZip`). */
    public function getHashZip(): ?string
    {
        return $this->optional('hash_zip');
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
