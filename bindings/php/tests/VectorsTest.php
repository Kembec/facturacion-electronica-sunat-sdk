<?php

declare(strict_types=1);

namespace Kembec\SunatSdk\Tests;

use Kembec\SunatSdk\SigningKey;
use Kembec\SunatSdk\SunatSdk;
use PHPUnit\Framework\Attributes\DataProvider;

/**
 * The 22 fixtures, through the real binary, must give their vectors' XML, unsigned and signed, under
 * inclusive C14N, and their exact `DigestValue` and `SignatureValue`. The vectors come from another serializer
 * (declaration, CDATA, empty elements), so the bytes differ by design.
 */
final class VectorsTest extends BinaryTestCase
{
    /**
     * Vouchers (01, 03, 07, 08), daily summaries (`rc-`) and void communications (`ra-`).
     *
     * @return array<string, array{0: string}>
     */
    public static function fixtures(): array
    {
        $cases = [];
        foreach (glob(self::repositoryPath('fixtures') . '/*.json') ?: [] as $path) {
            $name = basename($path, '.json');
            $cases[$name] = [$name];
        }

        return $cases;
    }

    public function testThereAreEighteenFixtures(): void
    {
        self::assertCount(22, self::fixtures());
    }

    /**
     * @dataProvider fixtures
     */
    #[DataProvider('fixtures')]
    public function testBuildMatchesTheUnsignedVector(string $name): void
    {
        $result = $this->sdk()->build($this->fixture($name));

        self::assertSame(self::canonical($this->vector($name, 'unsigned')), self::canonical($result->getXml()));
        self::assertSame($this->expectedFileName($name), $result->getFileName());
    }

    /**
     * @dataProvider fixtures
     */
    #[DataProvider('fixtures')]
    public function testSignMatchesTheSignedVector(string $name): void
    {
        $sdk = $this->sdk();
        $unsigned = $sdk->build($this->fixture($name))->getXml();
        $signed = $sdk->sign($unsigned, self::testKey());
        $expected = $this->vector($name, 'signed');

        self::assertSame(self::canonical($expected), self::canonical($signed->getXml()));
        self::assertStringContainsString('<ds:DigestValue>' . $signed->getDigestValue() . '</ds:DigestValue>', $expected);
        self::assertStringContainsString('<ds:SignatureValue>' . $signed->getSignatureValue() . '</ds:SignatureValue>', $expected);
        self::assertStringContainsString('<ds:DigestValue>' . $signed->getDigestValue() . '</ds:DigestValue>', $signed->getXml());
        self::assertStringContainsString('<ds:SignatureValue>' . $signed->getSignatureValue() . '</ds:SignatureValue>', $signed->getXml());
    }

    /** Inclusive C14N 1.0 without comments, the form the signature is computed over. */
    private static function canonical(string $xml): string
    {
        $document = new \DOMDocument();
        self::assertTrue($document->loadXML($xml), 'XML does not parse');

        return (string) $document->C14N(false, false);
    }

    private function sdk(): SunatSdk
    {
        return new SunatSdk(self::binaryPath(), 30.0);
    }

    private static function testKey(): SigningKey
    {
        $dir = self::repositoryPath('tests/vectors/test-cert');

        return new SigningKey($dir . '/test-cert.pem', $dir . '/test-key.pem');
    }

    /** The document exactly as the fixture file has it, objects kept as objects. */
    private function fixture(string $name): string
    {
        return (string) file_get_contents(self::repositoryPath('fixtures/' . $name . '.json'));
    }

    private function vector(string $name, string $kind): string
    {
        return (string) file_get_contents(self::repositoryPath('tests/vectors/' . $name . '.' . $kind . '.xml'));
    }

    /** `<ruc>-<type>-<series>-<number>` or `<ruc>-RC|RA-<YYYYMMDD>-<number>`, from the fixture's own fields. */
    private function expectedFileName(string $name): string
    {
        $document = json_decode($this->fixture($name), true);
        $ruc = $document['issuer']['ruc'];

        switch ($document['kind']) {
            case 'summary':
                return implode('-', [$ruc, 'RC', str_replace('-', '', $document['generation_date']), $document['number']]);
            case 'voided':
                return implode('-', [$ruc, 'RA', str_replace('-', '', $document['communication_date']), $document['number']]);
            default:
                return implode('-', [$ruc, $document['document_type'], $document['series'], $document['number']]);
        }
    }
}
