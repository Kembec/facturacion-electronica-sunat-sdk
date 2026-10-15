<?php

declare(strict_types=1);

namespace Kembec\SunatSdk;

/**
 * Paths of the PEM certificate and its unencrypted RSA private key. The paths go to the binary
 * through its environment, never through argv.
 */
final class SigningKey
{
    /** @var string */
    private $certificatePath;

    /** @var string */
    private $privateKeyPath;

    public function __construct(
        string $certificatePath,
        #[\SensitiveParameter]
        string $privateKeyPath
    ) {
        if ($certificatePath === '' || $privateKeyPath === '') {
            throw new \InvalidArgumentException('certificate and private key paths must not be empty');
        }
        $this->certificatePath = $certificatePath;
        $this->privateKeyPath = $privateKeyPath;
    }

    /**
     * @return array<string, string>
     */
    public function toEnvironment(): array
    {
        return ['SUNAT_CERT_PATH' => $this->certificatePath, 'SUNAT_KEY_PATH' => $this->privateKeyPath];
    }

    /**
     * @return array<string, string>
     */
    public function __debugInfo(): array
    {
        return ['certificatePath' => '[redacted]', 'privateKeyPath' => '[redacted]'];
    }
}
