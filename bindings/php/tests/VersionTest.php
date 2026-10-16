<?php

declare(strict_types=1);

namespace Kembec\SunatSdk\Tests;

use Kembec\SunatSdk\SunatSdk;
use PHPUnit\Framework\TestCase;

/** The installer downloads the release `v<VERSION>`, so the PHP client must carry the Cargo workspace version. */
final class VersionTest extends TestCase
{
    public function testTheClientVersionIsTheCargoWorkspaceVersion(): void
    {
        $manifest = (string) file_get_contents(dirname(__DIR__, 3) . '/Cargo.toml');
        $matched = preg_match('/^\[workspace\.package\][^\[]*?^version\s*=\s*"([^"]+)"/ms', $manifest, $m);

        self::assertSame(1, $matched, 'Cargo.toml has no [workspace.package] version');
        self::assertSame($m[1], SunatSdk::VERSION);
    }
}
