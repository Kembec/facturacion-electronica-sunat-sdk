<?php

declare(strict_types=1);

namespace Kembec\SunatSdk\Exception;

/** No prebuilt binary exists for this operating system or architecture; it has to be built from source. */
class UnsupportedPlatformException extends InstallException
{
}
