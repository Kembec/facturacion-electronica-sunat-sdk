<?php

declare(strict_types=1);

namespace Kembec\SunatSdk;

/**
 * SOL user and password for sendBill and getStatusCdr. They reach the binary only through its
 * environment, and never appear in var_dump, print_r or exception traces of this object.
 */
final class SolCredentials
{
    /** @var string */
    private $user;

    /** @var string */
    private $password;

    /**
     * @param string $user the RUC followed by the SOL user (`20100066603MODDATOS`)
     */
    public function __construct(
        #[\SensitiveParameter]
        string $user,
        #[\SensitiveParameter]
        string $password
    ) {
        if ($user === '' || $password === '') {
            throw new \InvalidArgumentException('SOL user and password must not be empty');
        }
        $this->user = $user;
        $this->password = $password;
    }

    /**
     * @return array<string, string>
     */
    public function toEnvironment(): array
    {
        return ['SUNAT_SOL_USER' => $this->user, 'SUNAT_SOL_PASSWORD' => $this->password];
    }

    /**
     * @return array<string, string>
     */
    public function __debugInfo(): array
    {
        return ['user' => '[redacted]', 'password' => '[redacted]'];
    }
}
