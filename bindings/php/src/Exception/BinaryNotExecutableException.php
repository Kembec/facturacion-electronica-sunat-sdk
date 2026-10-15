<?php

declare(strict_types=1);

namespace Kembec\SunatSdk\Exception;

/** The binary exists but cannot be executed (permissions, or the process could not start). */
class BinaryNotExecutableException extends SunatSdkException
{
}
