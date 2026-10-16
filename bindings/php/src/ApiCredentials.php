<?php

declare(strict_types=1);

namespace Kembec\SunatSdk;

/**
 * Client id and secret of «Credenciales de API SUNAT» (SOL), for the Consulta Integrada (`validity`).
 * They are not the SOL user and password, and not the GRE ones. They reach the binary only through its
 * environment, and never appear in var_dump, print_r or exception traces of this object.
 */
final class ApiCredentials
{
    /** @var string */
    private $clientId;

    /** @var string */
    private $clientSecret;

    public function __construct(
        #[\SensitiveParameter]
        string $clientId,
        #[\SensitiveParameter]
        string $clientSecret
    ) {
        if ($clientId === '' || $clientSecret === '') {
            throw new \InvalidArgumentException('API client id and secret must not be empty');
        }
        $this->clientId = $clientId;
        $this->clientSecret = $clientSecret;
    }

    /**
     * @return array<string, string>
     */
    public function toEnvironment(): array
    {
        return ['SUNAT_API_CLIENT_ID' => $this->clientId, 'SUNAT_API_CLIENT_SECRET' => $this->clientSecret];
    }

    /**
     * @return array<string, string>
     */
    public function __debugInfo(): array
    {
        return ['clientId' => '[redacted]', 'clientSecret' => '[redacted]'];
    }
}
