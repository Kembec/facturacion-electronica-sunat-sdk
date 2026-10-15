<?php

declare(strict_types=1);

namespace Kembec\SunatSdk\Exception;

/**
 * The binary ran and refused the call: its exit code was not zero. The message is the `error` field
 * the binary wrote on stderr, with any secret passed to the process removed.
 */
class SdkErrorException extends SunatSdkException
{
    /** @var string */
    private $command;

    /** @var int */
    private $exitCode;

    public function __construct(string $message, string $command, int $exitCode)
    {
        parent::__construct($message);
        $this->command = $command;
        $this->exitCode = $exitCode;
    }

    public function getCommand(): string
    {
        return $this->command;
    }

    public function getExitCode(): int
    {
        return $this->exitCode;
    }
}
