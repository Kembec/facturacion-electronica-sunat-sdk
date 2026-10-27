package Kembec::SunatSdk::Native;

# The C ABI of bindings/ffi (docs/c-abi.md): one JSON request in, one JSON response out. Loaded on first use, so
# `use Kembec::SunatSdk` never fails on a machine where the library cannot load; the error comes with the first call.

use strict;
use warnings;

use Config ();
use File::Basename ();
use File::Spec ();
use FFI::Platypus 2.00 ();
use FFI::Platypus::Buffer ();
use FFI::Platypus::DL ();
use JSON::PP ();

use Kembec::SunatSdk::Error;
use Kembec::SunatSdk::Platform;

use constant {
    LIBRARY_ENV  => 'KEMBEC_SUNAT_SDK_LIBRARY',
    ABI_VERSION  => 1,
    STATUS_OK    => 0,
    STATUS_ERROR => 1,
};

my $JSON = JSON::PP->new->utf8->canonical->allow_nonref;
my $loaded;

# Where the library is read from: $KEMBEC_SUNAT_SDK_LIBRARY, or the one installed with this distribution.
sub library_path {
    my $override = $ENV{+LIBRARY_ENV};
    return $override if defined $override && length $override;
    my $file = Kembec::SunatSdk::Platform::library_file() || 'libsunat_sdk_ffi.so';
    my $here = File::Basename::dirname(File::Spec->rel2abs(__FILE__));
    return File::Spec->catfile($here, 'native', Kembec::SunatSdk::Platform::key(), $file);
}

# The SDK version of the loaded library ("1.0.0-rc.9").
sub version {
    _load();
    return _sunat_sdk_version();
}

# Runs one command. $input is the UTF-8 JSON text of the command's input; $flags maps the CLI's flag names to strings
# or whole numbers. Returns the parsed response of a status 0 and its text; dies with Error on status 1.
sub run {
    my ($command, $input, $flags) = @_;
    my ($text, $status) = _call($command, $input, $flags);
    my $response = _parse($command, $text);
    if ($status == STATUS_ERROR) {
        my $message = $response->{error};
        Kembec::SunatSdk::Error->throw("unexpected response: `error` is missing", command => $command)
            unless defined $message && !ref $message;
        Kembec::SunatSdk::Error->throw($message, command => $command);
    }
    Kembec::SunatSdk::Error->throw("the library answered status $status", command => $command)
        unless $status == STATUS_OK;
    return ($response, $text);
}

# The library stays loaded for the life of the process: its handle is never closed and FFI::Platypus holds none, so
# neither a destroyed object nor a thread that ends (each clones what it inherits) can unload it under another caller.
sub _load {
    return if $loaded;
    my $path = library_path();
    my $handle = FFI::Platypus::DL::dlopen($path, FFI::Platypus::DL::RTLD_PLATYPUS_DEFAULT());
    Kembec::SunatSdk::NativeLibraryError->throw(_not_found($path, FFI::Platypus::DL::dlerror() || 'not found'))
        unless $handle;
    my $ffi = FFI::Platypus->new(api => 2);
    my %signatures = (
        sunat_sdk_call         => [['string', 'opaque*'] => 'sint32'],
        sunat_sdk_free         => [['opaque'] => 'void'],
        sunat_sdk_response_len => [['opaque'] => 'size_t'],
        sunat_sdk_version      => [[] => 'string'],
        sunat_sdk_abi_version  => [[] => 'uint32'],
    );
    for my $name (sort keys %signatures) {
        my $address = FFI::Platypus::DL::dlsym($handle, $name);
        Kembec::SunatSdk::NativeLibraryError->throw(_not_found($path, "the symbol $name is missing"))
            unless $address;
        $ffi->attach([$address => "_$name"] => @{ $signatures{$name} });
    }
    my $abi = _sunat_sdk_abi_version();
    Kembec::SunatSdk::NativeLibraryError->throw(
        "the native library speaks ABI $abi and this module ABI " . ABI_VERSION
            . ': the module and its native library come from different versions')
        unless $abi == ABI_VERSION;
    $loaded = 1;
    return;
}

sub _not_found {
    my ($path, $cause) = @_;
    my $bits = $Config::Config{ptrsize} * 8;
    return "the native library $path could not be loaded into this process ($^O, $bits-bit): $cause. "
        . 'This distribution carries builds for ' . join(', ', @Kembec::SunatSdk::Platform::SUPPORTED)
        . '; elsewhere set ' . LIBRARY_ENV . ' to a build of bindings/ffi.';
}

# The request is laid out once in a buffer of its final size, so no copy of it is left behind by a reallocation; the
# buffer and the flags (the only piece with secrets) are zeroed after the call. The input carries none.
sub _call {
    my ($command, $input, $flags) = @_;
    _load();
    my $secret = $flags ? $JSON->encode($flags) : undef;
    my @pieces = ('{"command":', $JSON->encode($command), ',"input":', $input);
    push @pieces, ',"flags":', $secret if defined $secret;
    push @pieces, '}';
    my $size = 0;
    $size += length $_ for @pieces;
    my $request = "\0" x $size;
    my $offset = 0;
    for my $piece (@pieces) {
        substr($request, $offset, length $piece, $piece);
        $offset += length $piece;
    }
    my $response;
    my $status = eval { _sunat_sdk_call($request, \$response) };
    my $failure = $@;
    substr($request, 0, $size, "\0" x $size);
    substr($secret, 0, length $secret, "\0" x length $secret) if defined $secret;
    die $failure unless defined $status;
    return (_take($command, $response), $status);
}

sub _take {
    my ($command, $response) = @_;
    Kembec::SunatSdk::Error->throw('the library stored no response', command => $command) unless $response;
    my $length = _sunat_sdk_response_len($response);
    my $text = FFI::Platypus::Buffer::buffer_to_scalar($response, $length);
    _sunat_sdk_free($response);
    Kembec::SunatSdk::Error->throw("the library's response is not UTF-8", command => $command)
        unless utf8::decode($text);
    return $text;
}

sub _parse {
    my ($command, $text) = @_;
    my $value = eval { JSON::PP->new->decode($text) };
    Kembec::SunatSdk::Error->throw("the library's response is not JSON", command => $command)
        unless defined $value;
    Kembec::SunatSdk::Error->throw("the library's response is not a JSON object", command => $command)
        unless ref $value eq 'HASH';
    _characters($value);
    return $value;
}

# Every string of the response as characters with Perl's UTF-8 flag on, so that passing one back to the SDK (the XML of
# build to sign) is never mistaken for bytes.
sub _characters {
    my ($value) = @_;
    for my $item (ref $value eq 'HASH' ? values %$value : @$value) {
        if (ref $item eq 'HASH' || ref $item eq 'ARRAY') { _characters($item) }
        elsif (defined $item && !ref $item) { utf8::upgrade($item) }
    }
    return;
}

1;
