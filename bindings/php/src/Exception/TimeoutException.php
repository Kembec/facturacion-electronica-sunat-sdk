<?php

declare(strict_types=1);

namespace Kembec\SunatSdk\Exception;

/** The binary did not finish within the configured timeout and was killed. */
class TimeoutException extends SunatSdkException
{
}
