<?php

declare(strict_types=1);

namespace Kembec\SunatSdk\Tests;

use Kembec\SunatSdk\Process\BinaryRunner;
use Kembec\SunatSdk\SigningKey;
use Kembec\SunatSdk\SolCredentials;

/** Secrets travel by the child's environment only: never argv, never inherited, never dumped. */
final class EnvironmentTest extends BinaryTestCase
{
    /** Prints its argv and its whole environment as JSON strings. */
    private const ECHO_SCRIPT = 'cat >/dev/null; printf \'{"args":"%s","env":"%s"}\' "$*" "$(env | tr \'\n"\' \' \')"';

    public function testCredentialsGoByEnvironmentAndNotByArgv(): void
    {
        $runner = new BinaryRunner($this->fakeBinary(self::ECHO_SCRIPT), 5.0);
        $env = (new SolCredentials('20100066603MODDATOS', 'pw-very-secret'))->toEnvironment()
            + (new SigningKey('/keys/cert.pem', '/keys/private-key.pem'))->toEnvironment();

        $out = $runner->run('send', '{}', $env);

        self::assertSame('send', $out['args']);
        self::assertStringContainsString('SUNAT_SOL_PASSWORD=pw-very-secret', $out['env']);
        self::assertStringContainsString('SUNAT_KEY_PATH=/keys/private-key.pem', $out['env']);
    }

    public function testTheParentEnvironmentIsNotInherited(): void
    {
        putenv('SUNAT_SOL_PASSWORD=parent-secret');
        try {
            $out = (new BinaryRunner($this->fakeBinary(self::ECHO_SCRIPT), 5.0))->run('zip', '{}');
        } finally {
            putenv('SUNAT_SOL_PASSWORD');
        }

        self::assertStringNotContainsString('parent-secret', $out['env']);
    }

    public function testDumpsHideCredentials(): void
    {
        $dump = print_r(new SolCredentials('20100066603MODDATOS', 'pw-very-secret'), true)
            . print_r(new SigningKey('/keys/cert.pem', '/keys/private-key.pem'), true);

        self::assertStringNotContainsString('pw-very-secret', $dump);
        self::assertStringNotContainsString('MODDATOS', $dump);
        self::assertStringNotContainsString('private-key.pem', $dump);
    }
}
