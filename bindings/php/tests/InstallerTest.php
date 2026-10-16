<?php

declare(strict_types=1);

namespace Kembec\SunatSdk\Tests;

use Kembec\SunatSdk\Exception\InstallException;
use Kembec\SunatSdk\Exception\UnsupportedPlatformException;
use Kembec\SunatSdk\Install\BinaryInstaller;
use Kembec\SunatSdk\SunatSdk;
use PHPUnit\Framework\TestCase;

/**
 * The installer against a release served by `php -S` on loopback: no network. `/good` publishes a binary
 * and its matching SHA-256, `/bad` a binary whose `.sha256` belongs to other bytes.
 */
final class InstallerTest extends TestCase
{
    private const BINARY = "#!/bin/sh\necho installed-sunat-sdk\n";

    /** @var resource|null */
    private static $server;

    /** @var string */
    private static $origin = '';

    /** @var string */
    private static $root = '';

    /** @var string */
    private $directory = '';

    public static function setUpBeforeClass(): void
    {
        self::$root = self::temporaryDirectory('sunat-sdk-release-');
        self::publish('good', self::BINARY, hash('sha256', self::BINARY));
        self::publish('bad', self::BINARY, hash('sha256', 'other bytes'));

        $port = self::freePort();
        self::$origin = '127.0.0.1:' . $port;
        $command = [PHP_BINARY, '-S', self::$origin, __DIR__ . '/release-server.php'];
        self::$server = proc_open($command, [0 => ['pipe', 'r'], 1 => ['file', '/dev/null', 'w'], 2 => ['file', '/dev/null', 'w']], $pipes, null, ['RELEASE_ROOT' => self::$root]);
        self::waitForServer($port);
    }

    public static function tearDownAfterClass(): void
    {
        if (is_resource(self::$server)) {
            proc_terminate(self::$server, 9);
            proc_close(self::$server);
        }
        self::removeTree(self::$root);
    }

    protected function setUp(): void
    {
        $this->directory = self::temporaryDirectory('sunat-sdk-install-');
    }

    protected function tearDown(): void
    {
        self::removeTree($this->directory);
    }

    public function testAMatchingSha256LeavesAnExecutableBinary(): void
    {
        $path = $this->installer('/good')->install($this->directory);

        self::assertSame(realpath($this->directory) . '/sunat-sdk', $path);
        self::assertSame(self::BINARY, file_get_contents($path));
        self::assertTrue(is_executable($path));
        self::assertSame(['sunat-sdk'], self::entries($this->directory));
    }

    public function testAMismatchedSha256LeavesNothing(): void
    {
        try {
            $this->installer('/bad')->install($this->directory);
            self::fail('a mismatched SHA-256 must throw');
        } catch (InstallException $e) {
            self::assertStringContainsString('SHA-256 mismatch', $e->getMessage());
        }

        self::assertSame([], self::entries($this->directory));
    }

    public function testAnUnsupportedPlatformExplainsHowToBuildIt(): void
    {
        $installer = new BinaryInstaller('http://' . self::$origin . '/good', 'Darwin', 'arm64');

        try {
            $installer->install($this->directory);
            self::fail('an unsupported platform must throw');
        } catch (UnsupportedPlatformException $e) {
            self::assertStringContainsString('Darwin arm64', $e->getMessage());
            self::assertStringContainsString('cargo build -p sunat-sdk-cli --release --locked', $e->getMessage());
            self::assertStringContainsString('--branch v' . SunatSdk::VERSION, $e->getMessage());
        }

        self::assertSame([], self::entries($this->directory));
    }

    public function testItFollowsARedirectLikeGitHubDoes(): void
    {
        $path = $this->installer('/redirect/' . self::$origin . '/good')->install($this->directory);

        self::assertSame(self::BINARY, file_get_contents($path));
    }

    public function testARedirectToPlainHttpOnARemoteHostIsRefused(): void
    {
        try {
            $this->installer('/redirect/example.com/good')->install($this->directory);
            self::fail('a redirect to plain HTTP on a remote host must throw');
        } catch (InstallException $e) {
            self::assertStringContainsString('only HTTPS is allowed', $e->getMessage());
        }

        self::assertSame([], self::entries($this->directory));
    }

    public function testAMissingReleaseFails(): void
    {
        $this->expectException(InstallException::class);
        $this->expectExceptionMessage('HTTP 404');

        $this->installer('/missing')->install($this->directory);
    }

    public function testTheDefaultSourceIsTheGitHubReleaseOfThisVersion(): void
    {
        self::assertSame(
            'https://github.com/Kembec/facturacion-electronica-sunat-sdk/releases/download/v' . SunatSdk::VERSION
            . '/sunat-sdk-x86_64-unknown-linux-musl.sha256',
            (new BinaryInstaller())->releaseUrl(BinaryInstaller::ASSET . '.sha256')
        );
    }

    private function installer(string $path): BinaryInstaller
    {
        return new BinaryInstaller('http://' . self::$origin . $path, 'Linux', 'x86_64');
    }

    private static function publish(string $name, string $binary, string $sha256): void
    {
        $release = self::$root . '/' . $name . '/v' . SunatSdk::VERSION;
        mkdir($release, 0755, true);
        file_put_contents($release . '/' . BinaryInstaller::ASSET, $binary);
        file_put_contents($release . '/' . BinaryInstaller::ASSET . '.sha256', $sha256 . '  ' . BinaryInstaller::ASSET . "\n");
    }

    private static function freePort(): int
    {
        $socket = stream_socket_server('tcp://127.0.0.1:0');
        $name = (string) stream_socket_get_name($socket, false);
        fclose($socket);

        return (int) substr($name, strrpos($name, ':') + 1);
    }

    private static function waitForServer(int $port): void
    {
        for ($i = 0; $i < 100; $i++) {
            $connection = @fsockopen('127.0.0.1', $port, $errno, $errstr, 0.1);
            if ($connection !== false) {
                fclose($connection);

                return;
            }
            usleep(50000);
        }
        self::fail('the local release server did not start');
    }

    private static function temporaryDirectory(string $prefix): string
    {
        $path = sys_get_temp_dir() . '/' . $prefix . bin2hex(random_bytes(6));
        mkdir($path, 0755, true);

        return $path;
    }

    /** @return string[] */
    private static function entries(string $directory): array
    {
        return array_values(array_diff(scandir($directory) ?: [], ['.', '..']));
    }

    private static function removeTree(string $path): void
    {
        if ($path === '' || !file_exists($path)) {
            return;
        }
        if (is_dir($path) && !is_link($path)) {
            foreach (array_diff(scandir($path) ?: [], ['.', '..']) as $entry) {
                self::removeTree($path . '/' . $entry);
            }
            rmdir($path);

            return;
        }
        unlink($path);
    }
}
