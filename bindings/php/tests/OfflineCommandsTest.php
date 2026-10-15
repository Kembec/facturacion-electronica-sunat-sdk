<?php

declare(strict_types=1);

namespace Kembec\SunatSdk\Tests;

use Kembec\SunatSdk\SunatSdk;

/** `zip` and `cdr` against the real binary; nothing here reaches SUNAT. */
final class OfflineCommandsTest extends BinaryTestCase
{
    private const CDR_XML = '<ar:ApplicationResponse xmlns:ar="urn:oasis:names:specification:ubl:schema:xsd:ApplicationResponse-2"'
        . ' xmlns:cac="urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2"'
        . ' xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2">'
        . '<cbc:Note>4252 - El dato ingresado como atributo @listName es incorrecto</cbc:Note>'
        . '<cac:DocumentResponse><cac:Response><cbc:ReferenceID>F001-1</cbc:ReferenceID>'
        . '<cbc:ResponseCode>0</cbc:ResponseCode><cbc:Description>La Factura numero F001-1, ha sido aceptada</cbc:Description>'
        . '</cac:Response></cac:DocumentResponse></ar:ApplicationResponse>';

    public function testZipNamesTheArchiveAfterTheDocument(): void
    {
        $result = (new SunatSdk(self::binaryPath()))->zip('20100066603-01-F001-1', '<Invoice/>');

        self::assertSame('20100066603-01-F001-1.zip', $result->getFileName());
        self::assertSame("PK\x03\x04", substr($result->getZipBytes(), 0, 4));
    }

    public function testCdrReadsAZippedConstancia(): void
    {
        $sdk = new SunatSdk(self::binaryPath());
        $zip = $sdk->zip('20100066603-01-F001-1', self::CDR_XML)->getZipBytes();

        $cdr = $sdk->cdr($zip);

        self::assertSame('F001-1', $cdr->getReferenceId());
        self::assertSame('0', $cdr->getResponseCode());
        self::assertSame('La Factura numero F001-1, ha sido aceptada', $cdr->getDescription());
        self::assertSame(['4252 - El dato ingresado como atributo @listName es incorrecto'], $cdr->getNotes());
        self::assertTrue($cdr->isAccepted());
    }
}
