<?php

declare(strict_types=1);

namespace Kembec\SunatSdk\Result;

use Kembec\SunatSdk\Exception\InvalidOutputException;

/** A code SUNAT answered, with the input fields that may have caused it (the binary's `diagnosis`). */
final class Diagnosis
{
    /** @var string */
    private $code;

    /** @var DiagnosisDetail|null */
    private $detail;

    /** @var DiagnosisField[] */
    private $fields;

    /**
     * @param DiagnosisField[] $fields
     */
    public function __construct(string $code, ?DiagnosisDetail $detail, array $fields)
    {
        $this->code = $code;
        $this->detail = $detail;
        $this->fields = $fields;
    }

    /**
     * @param array<string, mixed> $data
     */
    public static function fromArray(array $data): self
    {
        $detail = Fields::optionalObject($data, 'detail');

        return new self(
            Fields::string($data, 'code'),
            $detail === null ? null : DiagnosisDetail::fromArray($detail),
            array_map([DiagnosisField::class, 'fromArray'], self::objects($data, 'fields'))
        );
    }

    /**
     * The `diagnosis` of an outcome: one per code in the answer (the fault's, a CDR's non-zero
     * `response_code`, each note that starts with a code); empty when the output has none.
     *
     * @param array<string, mixed> $data
     * @return Diagnosis[]
     */
    public static function listFrom(array $data): array
    {
        return array_map([self::class, 'fromArray'], self::objects($data, 'diagnosis'));
    }

    public function getCode(): string
    {
        return $this->code;
    }

    /** What SUNAT's message names (node and value), or null when it names none. */
    public function getDetail(): ?DiagnosisDetail
    {
        return $this->detail;
    }

    /**
     * The candidate fields; empty for a code the rules do not list, and for summaries, void
     * communications and tickets. With a detail, only those on its node, unless none is.
     *
     * @return DiagnosisField[]
     */
    public function getFields(): array
    {
        return $this->fields;
    }

    /**
     * @param array<string, mixed> $data
     * @return array<int, array<string, mixed>>
     */
    private static function objects(array $data, string $key): array
    {
        $list = $data[$key] ?? [];
        if (!is_array($list)) {
            throw new InvalidOutputException(sprintf('sunat-sdk output field `%s` is not a list', $key));
        }
        foreach ($list as $item) {
            if (!is_array($item)) {
                throw new InvalidOutputException(sprintf('sunat-sdk output field `%s` has an element that is not an object', $key));
            }
        }

        return array_values($list);
    }
}
