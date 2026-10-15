<?php

declare(strict_types=1);

namespace Kembec\SunatSdk\Tests;

use Kembec\SunatSdk\SigningKey;
use Kembec\SunatSdk\SunatSdk;
use PHPUnit\Framework\Attributes\DataProvider;

/** The 12 fixtures, through the real binary, must give exactly the XML and signature of their vectors. */
final class VectorsTest extends BinaryTestCase
{
    /**
     * The voucher fixtures (01, 03, 07, 08); `ra-`/`rc-` summaries are not `build` input.
     *
     * @return array<string, array{0: string}>
     */
    public static function fixtures(): array
    {
        $cases = [];
        foreach (glob(self::repositoryPath('fixtures') . '/0[1378]-*.json') ?: [] as $path) {
            $name = basename($path, '.json');
            $cases[$name] = [$name];
        }

        return $cases;
    }

    public function testThereAreTwelveFixtures(): void
    {
        self::assertCount(12, self::fixtures());
    }

    /**
     * @dataProvider fixtures
     */
    #[DataProvider('fixtures')]
    public function testBuildMatchesTheUnsignedVector(string $name): void
    {
        $result = $this->sdk()->build($this->fixture($name));

        self::assertSame($this->vector($name, 'unsigned'), $result->getXml());
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

        self::assertSame($expected, $signed->getXml());
        self::assertStringContainsString('<ds:DigestValue>' . $signed->getDigestValue() . '</ds:DigestValue>', $expected);
        self::assertStringContainsString('<ds:SignatureValue>' . $signed->getSignatureValue() . '</ds:SignatureValue>', $expected);
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

    /** `<ruc>-<type>-<series>-<number>`, from the fixture's own fields. */
    private function expectedFileName(string $name): string
    {
        $document = json_decode($this->fixture($name), true);

        return implode('-', [$document['issuer']['ruc'], $document['document_type'], $document['series'], $document['number']]);
    }
}
