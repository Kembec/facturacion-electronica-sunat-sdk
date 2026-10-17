<?php

declare(strict_types=1);

namespace Kembec\SunatSdk\Tests;

use Kembec\SunatSdk\Exception\InvalidOutputException;
use Kembec\SunatSdk\Result\Diagnosis;
use Kembec\SunatSdk\Result\DiagnosisField;
use Kembec\SunatSdk\SolCredentials;
use Kembec\SunatSdk\SunatSdk;

/**
 * getDiagnosis() on send, status, summary and ticket. A fake binary prints the CLI's output for one SOAP
 * answer: `tests/data/*.json`, which the wasm binding's tests check against the core for the same answer
 * (bindings/wasm/src/api_tests.rs). send-beta-2800 is the beta's real answer to a boleta (17/10/2026).
 */
final class DiagnosisTest extends BinaryTestCase
{
    private const ENDPOINT = 'https://sunat.test/billService';

    public function testTheBeta2800PointsAtTheCustomerDocumentType(): void
    {
        $sent = $this->printing($this->fixture('send-beta-2800'))->send('20100066603-03-B001-1', '<Invoice/>', $this->credentials(), self::ENDPOINT);

        self::assertTrue($sent->isFault());
        self::assertSame('2800', $sent->getFault()->getCode());
        $diagnosis = $sent->getDiagnosis();
        self::assertCount(1, $diagnosis);
        self::assertInstanceOf(Diagnosis::class, $diagnosis[0]);
        self::assertSame('2800', $diagnosis[0]->getCode());
        $detail = $diagnosis[0]->getDetail();
        self::assertNotNull($detail);
        self::assertSame(['cbc:ID/schemeID', '1'], [$detail->getNode(), $detail->getValue()]);
        $fields = $diagnosis[0]->getFields();
        self::assertCount(1, $fields);
        self::assertSame('customer.document_type', $fields[0]->getInputPath());
        self::assertSame('/Invoice/cac:AccountingCustomerParty/cac:Party/cac:PartyIdentification/cbc:ID@schemeID', $fields[0]->getXpath());
        self::assertSame(DiagnosisField::SOURCE, $fields[0]->getCertainty());
    }

    public function testAnObservedCdrDiagnosesEachCodedNote(): void
    {
        $status = $this->printing($this->fixture('status-observed-4287'))->status('20100066603', '03', 'B001', 1, $this->credentials(), self::ENDPOINT);

        self::assertTrue($status->getCdr()->isValidDocument());
        $diagnosis = $status->getDiagnosis();
        self::assertSame(['4287', '4001'], array_map(static function (Diagnosis $d): string {
            return $d->getCode();
        }, $diagnosis));
        self::assertNull($diagnosis[0]->getDetail());
        self::assertSame('lines[].unit_price', $diagnosis[0]->getFields()[0]->getInputPath());
        self::assertSame(DiagnosisField::INFERRED, $diagnosis[0]->getFields()[0]->getCertainty());
        self::assertSame([], $diagnosis[1]->getFields(), '4001 is not in the 2026 rules');
    }

    public function testOutputWithoutDiagnosisGivesAnEmptyList(): void
    {
        $credentials = $this->credentials();
        $ticket = '{"outcome":"no_verdict","reason":"http_status","http_status":401,"message":"x","ticket":"1760620000123"}';

        self::assertSame([], $this->printing('{"outcome":"ticket","ticket":"1760620000123"}')->summary('20100066603-RC-20261016-1', '<a/>', $credentials, self::ENDPOINT)->getDiagnosis());
        self::assertSame([], $this->printing($ticket)->ticket('1760620000123', $credentials, self::ENDPOINT)->getDiagnosis());
        self::assertSame([], $this->printing('{"outcome":"fault","code":"0109","message":"x","class":null}')->send('20100066603-01-F001-1', '<a/>', $credentials, self::ENDPOINT)->getDiagnosis(), 'a binary before diagnosis');
        self::assertSame([], $this->printing('{"outcome":"status","status_code":"0001","status_message":"x"}')->status('20100066603', '01', 'F001', 1, $credentials, self::ENDPOINT)->getDiagnosis());
    }

    public function testASummaryAndATicketFaultCarryTheirDiagnosis(): void
    {
        $fault = '{"outcome":"fault","code":"2223","message":"x","class":"rejected","diagnosis":[{"code":"2223","detail":null,"fields":[]}]}';
        $credentials = $this->credentials();

        self::assertSame('2223', $this->printing($fault)->summary('20100066603-RC-20261016-1', '<a/>', $credentials, self::ENDPOINT)->getDiagnosis()[0]->getCode());
        self::assertSame([], $this->printing($fault)->ticket('1760620000123', $credentials, self::ENDPOINT)->getDiagnosis()[0]->getFields());
    }

    public function testAMalformedDiagnosisIsInvalidOutput(): void
    {
        $this->expectException(InvalidOutputException::class);
        $this->printing('{"outcome":"fault","code":"0109","message":"x","class":null,"diagnosis":[{"code":"0109","detail":null,"fields":"x"}]}')
            ->send('20100066603-01-F001-1', '<a/>', $this->credentials(), self::ENDPOINT);
    }

    private function fixture(string $name): string
    {
        return (string) file_get_contents(__DIR__ . '/data/' . $name . '.json');
    }

    private function credentials(): SolCredentials
    {
        return new SolCredentials('20100066603MODDATOS', 'moddatos');
    }

    private function printing(string $json): SunatSdk
    {
        return new SunatSdk($this->fakeBinary(sprintf('cat >/dev/null; printf \'%%s\' %s', escapeshellarg($json))), 90.0);
    }
}
