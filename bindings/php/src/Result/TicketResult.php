<?php

declare(strict_types=1);

namespace Kembec\SunatSdk\Result;

use Kembec\SunatSdk\Exception\InvalidOutputException;

/**
 * Output of `ticket` (getStatus of a summary's ticket): its state with the CDR when attached, a SOAP
 * fault, or no verdict when SUNAT answered an HTTP status instead of a response or the wait ran out.
 * Every one carries the ticket asked. In process (98) and no verdict mean ask again with the same
 * ticket, never send the summary again: the ticket proves SUNAT received it (docs/cli.md).
 */
final class TicketResult
{
    public const PROCESSED = 'processed';
    public const IN_PROCESS = 'in_process';
    public const PROCESSED_WITH_ERRORS = 'processed_with_errors';

    /** @var string */
    private $ticket;

    /** @var string|null */
    private $statusCode;

    /** @var string|null */
    private $state;

    /** @var Cdr|null */
    private $cdr;

    /** @var string|null */
    private $cdrZipBase64;

    /** @var Fault|null */
    private $fault;

    /** @var string|null `http_status` or `timeout` */
    private $noVerdictReason;

    /** @var int|null */
    private $httpStatus;

    /** @var string|null */
    private $message;

    /** @var Diagnosis[] */
    private $diagnosis = [];

    private function __construct(string $ticket)
    {
        $this->ticket = $ticket;
    }

    /**
     * @param array<string, mixed> $data
     * @param string $ticket the ticket asked; `ticket_status` does not repeat it
     */
    public static function fromArray(array $data, string $ticket): self
    {
        $result = self::read($data, $ticket);
        $result->diagnosis = Diagnosis::listFrom($data);

        return $result;
    }

    /**
     * @param array<string, mixed> $data
     * @param string $ticket the ticket asked; `ticket_status` does not repeat it
     */
    private static function read(array $data, string $ticket): self
    {
        $result = new self($ticket);
        $outcome = Fields::string($data, 'outcome');
        if ($outcome === 'fault') {
            $result->fault = Fault::fromArray($data);
            return $result;
        }
        if ($outcome === 'no_verdict') {
            return self::noVerdict($result, $data);
        }
        if ($outcome !== 'ticket_status') {
            throw new InvalidOutputException(sprintf('sunat-sdk ticket returned an unknown outcome `%s`', $outcome));
        }
        $result->statusCode = Fields::string($data, 'status_code');
        $result->state = Fields::optionalString($data, 'state');
        $cdr = Fields::optionalObject($data, 'cdr');
        $result->cdr = $cdr === null ? null : Cdr::fromArray($cdr);
        $result->cdrZipBase64 = Fields::optionalString($data, 'cdr_zip_base64');

        return $result;
    }

    /**
     * @param array<string, mixed> $data
     */
    private static function noVerdict(self $result, array $data): self
    {
        $httpStatus = $data['http_status'] ?? null;
        if ($httpStatus !== null && !is_int($httpStatus)) {
            throw new InvalidOutputException('sunat-sdk ticket returned a non-integer `http_status`');
        }
        $echoed = Fields::optionalString($data, 'ticket');
        if ($echoed !== null && $echoed !== $result->ticket) {
            throw new InvalidOutputException('sunat-sdk ticket returned another ticket than the one asked');
        }
        $result->noVerdictReason = Fields::string($data, 'reason');
        $result->httpStatus = $httpStatus;
        $result->message = Fields::optionalString($data, 'message') ?? '';

        return $result;
    }

    /** The ticket asked, whatever the outcome. */
    public function getTicket(): string
    {
        return $this->ticket;
    }

    /** SUNAT answered the query with the summary's state. */
    public function hasStatus(): bool
    {
        return $this->statusCode !== null;
    }

    /** `statusCode` as SUNAT sent it (`0`, `98`, `99`). */
    public function getStatusCode(): ?string
    {
        return $this->statusCode;
    }

    /** One of the class constants, or null for another code or no status. */
    public function getState(): ?string
    {
        return $this->state;
    }

    /** 98: SUNAT is still processing the summary. Ask again later with the same ticket; never send again. */
    public function isInProcess(): bool
    {
        return $this->state === self::IN_PROCESS;
    }

    /** The CDR SUNAT attached, read like cdr(). */
    public function getCdr(): ?Cdr
    {
        return $this->cdr;
    }

    public function getCdrZipBase64(): ?string
    {
        return $this->cdrZipBase64;
    }

    public function isFault(): bool
    {
        return $this->fault !== null;
    }

    public function getFault(): ?Fault
    {
        return $this->fault;
    }

    /** SUNAT gave no answer (an HTTP status or a timeout): ask again with the same ticket. */
    public function isNoVerdict(): bool
    {
        return $this->noVerdictReason !== null;
    }

    /** `http_status` or `timeout` when there is no verdict, null otherwise. */
    public function getNoVerdictReason(): ?string
    {
        return $this->noVerdictReason;
    }

    /** The HTTP status SUNAT answered instead of a response, when that is why there is no verdict. */
    public function getHttpStatus(): ?int
    {
        return $this->httpStatus;
    }

    /** Why there is no verdict, in the binary's words; null when there is a verdict. */
    public function getMessage(): ?string
    {
        return $this->message;
    }

    /**
     * The input field each code in the answer points at, one Diagnosis per code: the fault's, a CDR's
     * non-zero response code and each note that starts with a code. Empty when the binary sends none
     * (no verdict, or an answer without codes).
     *
     * @return Diagnosis[]
     */
    public function getDiagnosis(): array
    {
        return $this->diagnosis;
    }
}
