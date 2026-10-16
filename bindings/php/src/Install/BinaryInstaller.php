<?php

declare(strict_types=1);

namespace Kembec\SunatSdk\Install;

use Kembec\SunatSdk\Exception\InstallException;
use Kembec\SunatSdk\Exception\UnsupportedPlatformException;
use Kembec\SunatSdk\SunatSdk;

/**
 * Downloads the static Linux x86_64 `sunat-sdk` from the GitHub release `v<SunatSdk::VERSION>` and its
 * `.sha256` (the assets `.github/workflows/release.yml` publishes), and leaves it executable only if the
 * SHA-256 matches. The download goes to a temporary file in the target directory and is renamed into
 * place at the end, so a failed or tampered download never replaces or leaves a binary.
 */
final class BinaryInstaller
{
    public const ASSET = 'sunat-sdk-x86_64-unknown-linux-musl';
    public const BINARY_NAME = 'sunat-sdk';
    public const RELEASE_BASE_URL = 'https://github.com/Kembec/facturacion-electronica-sunat-sdk/releases/download';

    private const MAX_BINARY_BYTES = 268435456;
    private const MAX_CHECKSUM_BYTES = 4096;

    /** @var string */
    private $baseUrl;

    /** @var string */
    private $osFamily;

    /** @var string */
    private $machine;

    /** @var HttpFetcher */
    private $fetcher;

    /**
     * The arguments exist for the tests (a local release server, another platform); the
     * `sunat-sdk-install` command uses the defaults.
     */
    public function __construct(
        string $baseUrl = self::RELEASE_BASE_URL,
        ?string $osFamily = null,
        ?string $machine = null,
        ?HttpFetcher $fetcher = null
    ) {
        $this->baseUrl = rtrim($baseUrl, '/');
        $this->osFamily = $osFamily ?? PHP_OS_FAMILY;
        $this->machine = $machine ?? php_uname('m');
        $this->fetcher = $fetcher ?? new HttpFetcher();
    }

    public static function defaultDirectory(): string
    {
        return dirname(__DIR__, 2) . '/bin';
    }

    public function releaseUrl(string $file): string
    {
        return sprintf('%s/v%s/%s', $this->baseUrl, SunatSdk::VERSION, $file);
    }

    /**
     * @return string the absolute path of the verified, executable binary
     * @throws InstallException
     */
    public function install(string $directory): string
    {
        $this->assertSupportedPlatform();
        $expected = $this->expectedSha256();
        $directory = self::prepareDirectory($directory);

        $temporary = tempnam($directory, '.sunat-sdk-download-');
        if ($temporary === false) {
            throw new InstallException(sprintf('cannot create a temporary file in %s', $directory));
        }
        try {
            $actual = $this->download($temporary);
            if (!hash_equals($expected, $actual)) {
                throw new InstallException(sprintf(
                    'SHA-256 mismatch for %s: expected %s, got %s; nothing was installed',
                    $this->releaseUrl(self::ASSET),
                    $expected,
                    $actual
                ));
            }
            $target = $directory . DIRECTORY_SEPARATOR . self::BINARY_NAME;
            if (!chmod($temporary, 0755) || !rename($temporary, $target)) {
                throw new InstallException(sprintf('cannot move the binary into %s', $target));
            }
        } finally {
            if (is_file($temporary)) {
                unlink($temporary);
            }
        }

        return $target;
    }

    private function assertSupportedPlatform(): void
    {
        if ($this->osFamily === 'Linux' && in_array(strtolower($this->machine), ['x86_64', 'amd64'], true)) {
            return;
        }

        throw new UnsupportedPlatformException(sprintf(
            "There is no prebuilt sunat-sdk for %s %s; only Linux x86_64 is published.\n"
            . "Build it from source with Rust (the toolchain the repository CI pins):\n"
            . "  git clone --branch v%s https://github.com/Kembec/facturacion-electronica-sunat-sdk\n"
            . "  cd facturacion-electronica-sunat-sdk && cargo build -p sunat-sdk-cli --release --locked\n"
            . "then pass target/release/sunat-sdk to new SunatSdk(...) or export SUNAT_SDK_BIN with that path.",
            $this->osFamily,
            $this->machine,
            SunatSdk::VERSION
        ));
    }

    private function expectedSha256(): string
    {
        $memory = fopen('php://memory', 'w+b');
        $this->fetcher->fetch($this->releaseUrl(self::ASSET . '.sha256'), $memory, self::MAX_CHECKSUM_BYTES);
        rewind($memory);
        $content = (string) stream_get_contents($memory);
        fclose($memory);

        // sha256sum format: "<64 hex>  <file name>", one line.
        if (preg_match('/^([0-9a-fA-F]{64})\s+\*?(\S+)\s*$/', $content, $m) !== 1 || $m[2] !== self::ASSET) {
            throw new InstallException(sprintf('%s is not a sha256sum line for %s', $this->releaseUrl(self::ASSET . '.sha256'), self::ASSET));
        }

        return strtolower($m[1]);
    }

    /** @return string the SHA-256 of what was written */
    private function download(string $path): string
    {
        $sink = fopen($path, 'wb');
        if ($sink === false) {
            throw new InstallException(sprintf('cannot write %s', $path));
        }
        try {
            $this->fetcher->fetch($this->releaseUrl(self::ASSET), $sink, self::MAX_BINARY_BYTES);
        } finally {
            fclose($sink);
        }

        return (string) hash_file('sha256', $path);
    }

    private static function prepareDirectory(string $directory): string
    {
        if (!is_dir($directory) && !mkdir($directory, 0755, true) && !is_dir($directory)) {
            throw new InstallException(sprintf('cannot create %s', $directory));
        }
        $real = realpath($directory);
        if ($real === false || !is_writable($real)) {
            throw new InstallException(sprintf('%s is not writable', $directory));
        }

        return $real;
    }
}
