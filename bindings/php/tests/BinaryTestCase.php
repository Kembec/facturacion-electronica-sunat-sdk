<?php

declare(strict_types=1);

namespace Kembec\SunatSdk\Tests;

use PHPUnit\Framework\TestCase;

/** Locates the real binary and the repository fixtures, and creates throwaway fake binaries. */
abstract class BinaryTestCase extends TestCase
{
    /** @var string[] */
    private $tempFiles = [];

    protected static function binaryPath(): string
    {
        $configured = getenv('SUNAT_SDK_BIN');
        if (is_string($configured) && $configured !== '') {
            return $configured;
        }

        return dirname(__DIR__, 3) . '/target/debug/sunat-sdk';
    }

    protected static function repositoryPath(string $relative): string
    {
        return dirname(__DIR__, 3) . '/' . $relative;
    }

    /** A shell script standing in for the binary; `$mode` is its permission bits. */
    protected function fakeBinary(string $script, int $mode = 0755): string
    {
        $path = tempnam(sys_get_temp_dir(), 'sunat-sdk-fake-');
        file_put_contents($path, "#!/bin/sh\n" . $script . "\n");
        chmod($path, $mode);
        $this->tempFiles[] = $path;

        return $path;
    }

    protected function tearDown(): void
    {
        foreach ($this->tempFiles as $path) {
            @unlink($path);
        }
        $this->tempFiles = [];
        parent::tearDown();
    }
}
