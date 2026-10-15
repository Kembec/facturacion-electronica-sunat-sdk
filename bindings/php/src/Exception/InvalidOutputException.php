<?php

declare(strict_types=1);

namespace Kembec\SunatSdk\Exception;

/** The binary exited with success but its stdout is not the JSON object the contract promises. */
class InvalidOutputException extends SunatSdkException
{
}
