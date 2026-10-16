<?php

declare(strict_types=1);

namespace Kembec\SunatSdk\Install;

use Kembec\SunatSdk\Exception\InstallException;

/**
 * GET over HTTPS with the peer certificate and host name always verified, following redirects by hand
 * so that none can downgrade to plain HTTP. Plain HTTP is accepted only for a loopback host, which is
 * how the tests serve releases without network; a remote host is always HTTPS.
 */
final class HttpFetcher
{
    private const MAX_REDIRECTS = 5;
    private const CHUNK = 65536;
    private const LOOPBACK_HOSTS = ['127.0.0.1', 'localhost', '[::1]'];

    /** @var float */
    private $timeoutSeconds;

    public function __construct(float $timeoutSeconds = 60.0)
    {
        $this->timeoutSeconds = $timeoutSeconds;
    }

    /**
     * Streams the body into `$sink` and returns its byte count.
     *
     * @param resource $sink
     * @throws InstallException
     */
    public function fetch(string $url, $sink, int $maxBytes): int
    {
        for ($hop = 0; $hop <= self::MAX_REDIRECTS; $hop++) {
            $stream = $this->open($url);
            $headers = stream_get_meta_data($stream)['wrapper_data'] ?? [];
            $status = self::lastStatus($headers);

            if ($status >= 300 && $status < 400) {
                fclose($stream);
                $url = self::resolve($url, self::lastHeader($headers, 'location'));
                continue;
            }
            if ($status !== 200) {
                fclose($stream);
                throw new InstallException(sprintf('GET %s answered HTTP %d', $url, $status));
            }

            return self::copy($stream, $sink, $url, $maxBytes);
        }

        throw new InstallException(sprintf('more than %d redirects downloading %s', self::MAX_REDIRECTS, $url));
    }

    /** @return resource */
    private function open(string $url)
    {
        $this->assertAllowed($url);
        $context = stream_context_create([
            'http' => [
                'method' => 'GET',
                'follow_location' => 0,
                'ignore_errors' => true,
                'timeout' => $this->timeoutSeconds,
                'user_agent' => 'kembec-sunat-sdk-installer',
            ],
            'ssl' => [
                'verify_peer' => true,
                'verify_peer_name' => true,
                'allow_self_signed' => false,
                'SNI_enabled' => true,
            ],
        ]);

        $errors = [];
        set_error_handler(static function (int $severity, string $message) use (&$errors): bool {
            $errors[] = $message;

            return true;
        });
        try {
            $stream = fopen($url, 'rb', false, $context);
        } finally {
            restore_error_handler();
        }
        if ($stream === false) {
            throw new InstallException(sprintf('could not download %s: %s', $url, $errors === [] ? 'unknown error' : implode('; ', $errors)));
        }

        return $stream;
    }

    private function assertAllowed(string $url): void
    {
        $scheme = strtolower((string) parse_url($url, PHP_URL_SCHEME));
        $host = strtolower((string) parse_url($url, PHP_URL_HOST));

        if ($scheme === 'https') {
            if (!extension_loaded('openssl')) {
                throw new InstallException('the openssl extension is required to download over HTTPS');
            }

            return;
        }
        if ($scheme === 'http' && in_array($host, self::LOOPBACK_HOSTS, true)) {
            return;
        }

        throw new InstallException(sprintf('refusing %s: only HTTPS is allowed', $url));
    }

    /**
     * @param resource $stream
     * @param resource $sink
     */
    private static function copy($stream, $sink, string $url, int $maxBytes): int
    {
        $total = 0;
        try {
            while (!feof($stream)) {
                $chunk = fread($stream, self::CHUNK);
                if ($chunk === false) {
                    throw new InstallException(sprintf('read error downloading %s', $url));
                }
                $total += strlen($chunk);
                if ($total > $maxBytes) {
                    throw new InstallException(sprintf('%s is larger than %d bytes', $url, $maxBytes));
                }
                if ($chunk !== '' && fwrite($sink, $chunk) !== strlen($chunk)) {
                    throw new InstallException(sprintf('write error saving %s', $url));
                }
            }
            if (!empty(stream_get_meta_data($stream)['timed_out'])) {
                throw new InstallException(sprintf('timed out downloading %s', $url));
            }
        } finally {
            fclose($stream);
        }

        return $total;
    }

    /** @param string[] $headers every response of the request, redirects included */
    private static function lastStatus(array $headers): int
    {
        $status = 0;
        foreach ($headers as $line) {
            if (preg_match('#^HTTP/\S+\s+(\d{3})#', $line, $m) === 1) {
                $status = (int) $m[1];
            }
        }

        return $status;
    }

    /** @param string[] $headers */
    private static function lastHeader(array $headers, string $name): string
    {
        $value = '';
        foreach ($headers as $line) {
            $parts = explode(':', $line, 2);
            if (count($parts) === 2 && strtolower(trim($parts[0])) === $name) {
                $value = trim($parts[1]);
            }
        }
        if ($value === '') {
            throw new InstallException('redirect without a Location header');
        }

        return $value;
    }

    private static function resolve(string $base, string $location): string
    {
        if (preg_match('#^[a-z][a-z0-9+.-]*://#i', $location) === 1) {
            return $location;
        }
        $origin = parse_url($base, PHP_URL_SCHEME) . '://' . parse_url($base, PHP_URL_HOST);
        $port = parse_url($base, PHP_URL_PORT);
        if ($port !== null && $port !== false) {
            $origin .= ':' . $port;
        }
        if (strpos($location, '/') === 0) {
            return $origin . $location;
        }

        throw new InstallException(sprintf('unsupported relative redirect %s', $location));
    }
}
