<?php

declare(strict_types=1);

namespace Kembec\SunatSdk\Tests;

use Kembec\SunatSdk\SigningKey;
use Kembec\SunatSdk\SolCredentials;
use Kembec\SunatSdk\SunatSdk;

/**
 * What a consumer with Syma's storage and queue needs: one combined PEM as certificate and key, an
 * HTTP wait it bounds below its worker timeout, and a resend that carries the stored signed XML as is.
 */
final class SymaCompatTest extends BinaryTestCase
{
    /** @var string[] */
    private $files = [];

    public function testOneCombinedPemServesAsCertificateAndKey(): void
    {
        $dir = self::repositoryPath('tests/vectors');
        $cert = (string) file_get_contents($dir . '/test-cert/test-cert.pem');
        $key = (string) file_get_contents($dir . '/test-cert/test-key.pem');
        $expected = (string) file_get_contents($dir . '/01-invoice-taxed-cash.signed.xml');
        $unsigned = (string) file_get_contents($dir . '/01-invoice-taxed-cash.unsigned.xml');
        $sdk = new SunatSdk(self::binaryPath(), 30.0);

        foreach (['cert then key' => $cert . $key, 'key then cert, CRLF' => str_replace("\n", "\r\n", $key . $cert)] as $layout => $pem) {
            $combined = $this->file($pem);
            $signed = $sdk->sign($unsigned, new SigningKey($combined, $combined));

            self::assertStringContainsString('<ds:DigestValue>' . $signed->getDigestValue() . '</ds:DigestValue>', $expected, $layout);
            self::assertStringContainsString('<ds:SignatureValue>' . $signed->getSignatureValue() . '</ds:SignatureValue>', $expected, $layout);
        }
    }

    public function testHttpTimeoutDefaultsAndBounds(): void
    {
        $bin = self::binaryPath();
        self::assertSame(90, (new SunatSdk($bin))->httpTimeoutSeconds());
        self::assertSame(55, (new SunatSdk($bin, 60.0))->httpTimeoutSeconds());
        self::assertSame(45, (new SunatSdk($bin, 55.0, 45))->httpTimeoutSeconds());
        self::assertSame(50, (new SunatSdk($bin, 55.0, 50))->httpTimeoutSeconds());
        self::assertNull((new SunatSdk($bin, 0.3))->httpTimeoutSeconds());

        foreach ([[55.0, 51], [55.0, 0], [400.0, 301]] as [$process, $http]) {
            try {
                new SunatSdk($bin, $process, $http);
                self::fail(sprintf('accepted %d s inside %.1f s', $http, $process));
            } catch (\InvalidArgumentException $e) {
                self::assertStringContainsString('httpTimeoutSeconds', $e->getMessage());
            }
        }
    }

    public function testSendAndStatusPassTheHttpTimeoutToTheBinary(): void
    {
        $envFile = $this->file('');
        $fault = '{"outcome":"fault","code":"0109","message":"x","class":null}';
        $bin = $this->fakeBinary(sprintf('cat >/dev/null; env >> %s; printf \'%s\'', escapeshellarg($envFile), $fault));
        $sdk = new SunatSdk($bin, 55.0, 45);
        $credentials = new SolCredentials('20100066603MODDATOS', 'moddatos');

        $sdk->send('20100066603-01-F001-1', '<Invoice/>', $credentials, 'https://sunat.test/billService');
        self::assertStringContainsString("SUNAT_HTTP_TIMEOUT_SECONDS=45\n", (string) file_get_contents($envFile));

        file_put_contents($envFile, '');
        $statusBin = $this->fakeBinary(sprintf('cat >/dev/null; env >> %s; printf \'{"outcome":"status","status_code":"0001","status_message":"x"}\'', escapeshellarg($envFile)));
        (new SunatSdk($statusBin, 30.0))->status('20100066603', '01', 'F001', 1, $credentials, 'https://sunat.test/billConsultService');
        self::assertStringContainsString("SUNAT_HTTP_TIMEOUT_SECONDS=25\n", (string) file_get_contents($envFile));

        $this->expectException(\InvalidArgumentException::class);
        (new SunatSdk($bin, 0.3))->send('20100066603-01-F001-1', '<Invoice/>', $credentials, 'https://sunat.test/billService');
    }

    public function testResendCarriesTheStoredSignedXmlByteForByte(): void
    {
        $stored = str_replace("\n", "\r\n", (string) file_get_contents(self::repositoryPath('tests/vectors/03-receipt-consumer-dni.signed.xml')));
        $stdinFile = $this->file('');
        $fault = '{"outcome":"fault","code":"0109","message":"x","class":null}';
        $bin = $this->fakeBinary(sprintf('cat >> %s; printf \'%s\'', escapeshellarg($stdinFile), $fault));
        $sdk = new SunatSdk($bin, 30.0);
        $credentials = new SolCredentials('20000000001MODDATOS', 'moddatos');

        $sdk->send('20000000001-03-B001-1', $stored, $credentials, 'https://sunat.test/billService');
        $first = (string) file_get_contents($stdinFile);
        $sdk->send('20000000001-03-B001-1', $stored, $credentials, 'https://sunat.test/billService');
        $both = (string) file_get_contents($stdinFile);

        self::assertSame($first . $first, $both, 'the retry sends the same input');
        $input = json_decode($first, true);
        self::assertSame($stored, $input['xml']);
        self::assertSame('20000000001-03-B001-1', $input['name']);
    }

    private function file(string $content): string
    {
        $path = (string) tempnam(sys_get_temp_dir(), 'sunat-sdk-syma-');
        file_put_contents($path, $content);
        $this->files[] = $path;

        return $path;
    }

    protected function tearDown(): void
    {
        foreach ($this->files as $path) {
            @unlink($path);
        }
        $this->files = [];
        parent::tearDown();
    }
}
