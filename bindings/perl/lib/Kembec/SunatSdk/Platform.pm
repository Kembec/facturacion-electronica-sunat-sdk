package Kembec::SunatSdk::Platform;

# Which native build this process loads. Pure Perl, without FFI::Platypus, because Makefile.PL also asks it which
# library to install.

use strict;
use warnings;

use Config ();
use POSIX ();

my %LIBRARY_FILE = (
    linux   => 'libsunat_sdk_ffi.so',
    darwin  => 'libsunat_sdk_ffi.dylib',
    MSWin32 => 'sunat_sdk_ffi.dll',
);

# The builds this distribution carries, as .NET runtime identifiers.
our @SUPPORTED = qw(linux-x64 linux-arm64 osx-x64 osx-arm64 win-x64);

# The library's file name on this operating system; undef where the SDK has no build.
sub library_file { $LIBRARY_FILE{$^O} }

# "linux-x64", "osx-arm64", "win-x64"... The architecture is the process's, not the machine's: a 32-bit perl on a
# 64-bit kernel gives "x86", and a perl under Rosetta "x64".
sub key {
    my $os = $^O eq 'MSWin32' ? 'win' : $^O eq 'darwin' ? 'osx' : $^O;
    my $machine = lc((POSIX::uname())[4] || '');
    $machine = $Config::Config{archname} if $^O eq 'MSWin32';
    my $bits = $Config::Config{ptrsize} * 8;
    my $arch
        = $machine =~ /aarch64|arm64/ ? ($bits == 64 ? 'arm64' : 'arm')
        : $machine =~ /x86_64|amd64|x64|i[3-6]86|x86/ ? ($bits == 64 ? 'x64' : 'x86')
        : $machine;
    return "$os-$arch";
}

sub is_supported {
    my $key = key();
    return scalar grep { $_ eq $key } @SUPPORTED;
}

1;
