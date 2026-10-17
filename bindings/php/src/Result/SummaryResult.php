<?php

declare(strict_types=1);

namespace Kembec\SunatSdk\Result;

use Kembec\SunatSdk\Exception\InvalidOutputException;

/**
 * Output of `summary` (sendSummary of a daily summary RC or a void communication RA): the ticket, or the
 * SOAP fault SUNAT answered with. Store the ticket before anything else: it is the only handle on the
 * outcome. No answer is a SdkErrorException, not a result: the summary may have arrived, so check the
 * documents with validity() and resend the same file under the same name (docs/cli.md).
 */
final class SummaryResult
{
    /** @var string|null */
    private $ticket;

    /** @var Fault|null */
    private $fault;

    private function __construct(?string $ticket, ?Fault $fault)
    {
        $this->ticket = $ticket;
        $this->fault = $fault;
    }

    /**
     * @param array<string, mixed> $data
     */
    public static function fromArray(array $data): self
    {
        $outcome = Fields::string($data, 'outcome');
        if ($outcome === 'fault') {
            return new self(null, Fault::fromArray($data));
        }
        if ($outcome !== 'ticket') {
            throw new InvalidOutputException(sprintf('sunat-sdk summary returned an unknown outcome `%s`', $outcome));
        }

        return new self(Fields::string($data, 'ticket'), null);
    }

    /** SUNAT took the summary and gave a ticket to ask for its outcome with ticket(). */
    public function hasTicket(): bool
    {
        return $this->ticket !== null;
    }

    public function getTicket(): ?string
    {
        return $this->ticket;
    }

    public function isFault(): bool
    {
        return $this->fault !== null;
    }

    public function getFault(): ?Fault
    {
        return $this->fault;
    }
}
