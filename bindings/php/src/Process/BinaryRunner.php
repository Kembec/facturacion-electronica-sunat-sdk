<?php

declare(strict_types=1);

namespace Kembec\SunatSdk\Process;

use Kembec\SunatSdk\Exception\BinaryNotExecutableException;
use Kembec\SunatSdk\Exception\BinaryNotFoundException;
use Kembec\SunatSdk\Exception\InvalidOutputException;
use Kembec\SunatSdk\Exception\SdkErrorException;
use Kembec\SunatSdk\Exception\TimeoutException;

/**
 * Runs one `sunat-sdk <command>` call: JSON on stdin, JSON object on stdout, `{"error"}` on stderr.
 *
 * The child gets ONLY the environment passed to `run()` (nothing is inherited), so a credential in
 * the parent's environment never reaches a command that did not ask for it. Values in that
 * environment are secrets by contract: they never go in argv and are scrubbed from error messages.
 */
final class BinaryRunner
{
    private const CHUNK = 65536;

    /** @var string */
    private $binaryPath;

    /** @var float */
    private $timeoutSeconds;

    public function __construct(string $binaryPath, float $timeoutSeconds)
    {
        if ($timeoutSeconds <= 0) {
            throw new \InvalidArgumentException('timeout must be greater than zero');
        }
        $this->binaryPath = $binaryPath;
        $this->timeoutSeconds = $timeoutSeconds;
    }

    /**
     * @param array<string, string> $env environment of the child process (secrets go here)
     * @return array<string, mixed> the decoded stdout object
     */
    public function run(string $command, string $stdin, array $env = []): array
    {
        $this->assertRunnable();
        $process = $this->start($command, $env, $pipes);
        [$stdout, $stderr, $timedOut] = $this->exchange($process, $pipes, $stdin);

        if ($timedOut) {
            proc_terminate($process, 9);
            proc_close($process);
            throw new TimeoutException(sprintf('sunat-sdk %s exceeded %.1f s and was killed', $command, $this->timeoutSeconds));
        }

        $exitCode = proc_close($process);
        if ($exitCode !== 0) {
            throw new SdkErrorException($this->scrub($this->errorMessage($stderr, $exitCode), $env), $command, $exitCode);
        }

        return $this->decodeOutput($command, $stdout);
    }

    private function assertRunnable(): void
    {
        if (!is_file($this->binaryPath)) {
            throw new BinaryNotFoundException(sprintf('sunat-sdk binary not found at %s', $this->binaryPath));
        }
        if (!is_executable($this->binaryPath)) {
            throw new BinaryNotExecutableException(sprintf('sunat-sdk binary at %s is not executable', $this->binaryPath));
        }
    }

    /**
     * @param array<string, string> $env
     * @param array<int, resource>|null $pipes
     * @return resource
     */
    private function start(string $command, array $env, &$pipes)
    {
        $spec = [0 => ['pipe', 'r'], 1 => ['pipe', 'w'], 2 => ['pipe', 'w']];
        $process = @proc_open([$this->binaryPath, $command], $spec, $pipes, null, $env);
        if (!is_resource($process)) {
            throw new BinaryNotExecutableException(sprintf('could not start sunat-sdk at %s', $this->binaryPath));
        }
        foreach ($pipes as $pipe) {
            stream_set_blocking($pipe, false);
        }

        return $process;
    }

    /**
     * Writes stdin and drains stdout and stderr together, so a large output never blocks the child.
     *
     * @param resource $process
     * @param array<int, resource> $pipes
     * @return array{0: string, 1: string, 2: bool} stdout, stderr, timed out
     */
    private function exchange($process, array $pipes, string $stdin): array
    {
        $deadline = microtime(true) + $this->timeoutSeconds;
        $output = [1 => '', 2 => ''];
        $open = [1 => $pipes[1], 2 => $pipes[2]];
        $pending = $stdin;
        $stdinPipe = $pipes[0];
        if ($pending === '') {
            $stdinPipe = $this->closePipe($stdinPipe);
        }

        while ($open !== [] || $stdinPipe !== null) {
            $remaining = $deadline - microtime(true);
            if ($remaining <= 0) {
                $this->closeAll($open, $stdinPipe);
                return [$output[1], $output[2], true];
            }
            $read = array_values($open);
            $write = $stdinPipe !== null ? [$stdinPipe] : [];
            $except = null;
            $seconds = (int) $remaining;
            $ready = @stream_select($read, $write, $except, $seconds, (int) (($remaining - $seconds) * 1e6));
            if ($ready === false) {
                continue;
            }
            if ($write !== []) {
                $pending = $this->writeChunk($stdinPipe, $pending);
                if ($pending === null || $pending === '') {
                    $stdinPipe = $this->closePipe($stdinPipe);
                }
            }
            foreach ($open as $index => $pipe) {
                if (!in_array($pipe, $read, true)) {
                    continue;
                }
                $chunk = fread($pipe, self::CHUNK);
                if ($chunk !== false && $chunk !== '') {
                    $output[$index] .= $chunk;
                }
                if (feof($pipe)) {
                    fclose($pipe);
                    unset($open[$index]);
                }
            }
        }

        return [$output[1], $output[2], false];
    }

    /**
     * @param resource $pipe
     * @return string|null what is left to write, or null if the child closed its stdin
     */
    private function writeChunk($pipe, string $pending): ?string
    {
        $written = @fwrite($pipe, substr($pending, 0, self::CHUNK));
        if ($written === false) {
            return null;
        }

        return (string) substr($pending, $written);
    }

    /**
     * @param resource $pipe
     * @return null
     */
    private function closePipe($pipe)
    {
        @fclose($pipe);

        return null;
    }

    /**
     * @param array<int, resource> $open
     * @param resource|null $stdinPipe
     */
    private function closeAll(array $open, $stdinPipe): void
    {
        foreach ($open as $pipe) {
            @fclose($pipe);
        }
        if ($stdinPipe !== null) {
            @fclose($stdinPipe);
        }
    }

    private function errorMessage(string $stderr, int $exitCode): string
    {
        $decoded = json_decode(trim($stderr), true);
        if (is_array($decoded) && isset($decoded['error']) && is_string($decoded['error'])) {
            return $decoded['error'];
        }
        $raw = trim($stderr);
        if ($raw === '') {
            return sprintf('sunat-sdk exited with code %d and no message', $exitCode);
        }

        return sprintf('sunat-sdk exited with code %d: %s', $exitCode, substr($raw, 0, 500));
    }

    /**
     * @param array<string, string> $env
     */
    private function scrub(string $message, array $env): string
    {
        foreach ($env as $value) {
            if (strlen($value) >= 3) {
                $message = str_replace($value, '[redacted]', $message);
            }
        }

        return $message;
    }

    /**
     * @return array<string, mixed>
     */
    private function decodeOutput(string $command, string $stdout): array
    {
        $decoded = json_decode($stdout, true);
        if (!is_array($decoded)) {
            throw new InvalidOutputException(sprintf('sunat-sdk %s did not print a JSON object', $command));
        }

        return $decoded;
    }
}
