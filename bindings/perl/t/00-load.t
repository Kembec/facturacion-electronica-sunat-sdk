use strict;
use warnings;

use Test::More;

BEGIN { use_ok('Kembec::SunatSdk') }

# The library installed for this platform loads, speaks this ABI and is the SDK version of the module.
is(Kembec::SunatSdk->native_version, Kembec::SunatSdk::SDK_VERSION(), 'native version');
is(Kembec::SunatSdk::Native::_sunat_sdk_abi_version(), Kembec::SunatSdk::ABI_VERSION(), 'ABI version');
ok(-f Kembec::SunatSdk::Native::library_path(), 'native library ' . Kembec::SunatSdk::Native::library_path());
diag('platform ' . Kembec::SunatSdk::Platform::key() . ", perl $^V, FFI::Platypus $FFI::Platypus::VERSION");

done_testing;
