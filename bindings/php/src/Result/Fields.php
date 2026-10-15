<?php

declare(strict_types=1);

namespace Kembec\SunatSdk\Result;

use Kembec\SunatSdk\Exception\InvalidOutputException;

/** Reads the binary's output fields, failing with InvalidOutputException when the shape is wrong. */
final class Fields
{
    /**
     * @param array<string, mixed> $data
     */
    public static function string(array $data, string $key): string
    {
        if (!isset($data[$key]) || !is_string($data[$key])) {
            throw new InvalidOutputException(sprintf('sunat-sdk output lacks string field `%s`', $key));
        }

        return $data[$key];
    }

    /**
     * @param array<string, mixed> $data
     */
    public static function optionalString(array $data, string $key): ?string
    {
        if (!isset($data[$key])) {
            return null;
        }

        return self::string($data, $key);
    }

    /**
     * @param array<string, mixed> $data
     * @return array<string, mixed>|null
     */
    public static function optionalObject(array $data, string $key): ?array
    {
        if (!isset($data[$key])) {
            return null;
        }
        if (!is_array($data[$key])) {
            throw new InvalidOutputException(sprintf('sunat-sdk output field `%s` is not an object', $key));
        }

        return $data[$key];
    }
}
