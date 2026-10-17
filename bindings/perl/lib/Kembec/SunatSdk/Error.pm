package Kembec::SunatSdk::Error;

# A command that did not run: the SDK's own error (the CLI's {"error"} with exit code 1, such as an invalid document,
# a missing certificate or a send without an answer) or a response that is not the contract's. An answer from SUNAT,
# a rejection included, is a result, never this error.

use strict;
use warnings;

use overload '""' => sub { $_[0]->{message} }, bool => sub { 1 }, fallback => 1;

sub new {
    my ($class, $message, %options) = @_;
    return bless { message => $message, command => $options{command} }, $class;
}

sub throw {
    my $class = shift;
    die $class->new(@_);
}

sub message { $_[0]->{message} }

# The command of docs/cli.md that failed ("build", "send"...); undef if none ran.
sub command { $_[0]->{command} }

package Kembec::SunatSdk::NativeLibraryError;

# The native library was not found, could not be loaded into this process, or speaks another ABI. Nothing ran.

use strict;
use warnings;

our @ISA = ('Kembec::SunatSdk::Error');

package Kembec::SunatSdk::ArgumentError;

# An argument the SDK refuses before calling the library: missing, of another kind, empty, not UTF-8, a float amount.
# It reads like croak: the message, then where the caller's code made the call.

use strict;
use warnings;

use overload '""' => sub { $_[0]->{message} . $_[0]->{where} }, bool => sub { 1 }, fallback => 1;

sub new {
    my ($class, $message) = @_;
    return bless { message => $message, where => _where() }, $class;
}

sub throw {
    my $class = shift;
    die $class->new(@_);
}

sub message { $_[0]->{message} }

# The first frame outside this distribution: the caller's line, as croak would report it.
sub _where {
    for (my $level = 1; ; $level++) {
        my ($package, $file, $line) = caller $level;
        return "\n" unless defined $package;
        return " at $file line $line.\n" unless $package =~ /\AKembec::SunatSdk(?:::|\z)/;
    }
}

1;
