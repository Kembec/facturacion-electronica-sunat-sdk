package Kembec::SunatSdk::Arguments;

# Checks shared by the client and the credential objects. Every text reaches the SDK as UTF-8: a Perl character string
# (UTF-8 flag on, as the results of this module are) is encoded; a byte string, as read from a file without a layer, is
# taken as UTF-8 and refused if it is not. Text in another encoding is refused, never misread.

use strict;
use warnings;

use B ();
use overload ();
use Scalar::Util ();

use Kembec::SunatSdk::Error;

sub fail { Kembec::SunatSdk::ArgumentError->throw(@_) }

# A defined string, number or object that stringifies (Path::Tiny, Math::BigFloat), as Perl characters.
sub characters {
    my ($value, $name) = @_;
    fail("$name is required") unless defined $value;
    if (ref $value) {
        fail("$name must be a string, not a " . ref($value) . ' reference')
            unless Scalar::Util::blessed($value) && overload::Method($value, '""');
        $value = "$value";
    }
    return $value if utf8::is_utf8($value);
    my $copy = $value;
    fail("$name is not valid UTF-8") unless utf8::decode($copy);
    return $copy;
}

sub nonempty {
    my ($value, $name) = @_;
    my $text = characters($value, $name);
    fail("$name must not be empty") unless length $text;
    return $text;
}

sub optional { defined $_[0] ? characters(@_) : undef }

# The UTF-8 bytes of a text.
sub bytes {
    my $text = characters(@_);
    utf8::encode($text);
    return $text;
}

# A whole number from 0 up, as a Perl number (JSON encodes it as a number).
sub whole {
    my ($value, $name) = @_;
    fail("$name is required") unless defined $value;
    fail("$name must be a whole number, not «$value»") if ref $value || "$value" !~ /\A[0-9]+\z/;
    return 0 + $value;
}

# A float is refused: it is not an exact amount. A string, an integer or an object that stringifies to the amount
# (Math::BigFloat) goes as text; the SDK checks its digits.
sub amount {
    my ($value, $name) = @_;
    fail("$name is required") unless defined $value;
    fail("$name must be exact: pass a string such as \"118.00\" or a Math::BigFloat, not a floating-point number")
        if is_float($value);
    return characters($value, $name);
}

# A scalar that is only a floating-point number: never a string, never an integer.
sub is_float {
    my ($value) = @_;
    return 0 if ref $value;
    my $flags = B::svref_2object(\$value)->FLAGS;
    return ($flags & B::SVp_NOK) && !($flags & (B::SVp_POK | B::SVp_IOK)) ? 1 : 0;
}

# Named arguments: every required one present, none unknown (a typo would otherwise pass in silence).
sub named {
    my ($what, $arguments, $required, $optional) = @_;
    fail("$what takes named arguments (name => value)") if @$arguments % 2;
    my %given = @$arguments;
    my %known = map { $_ => 1 } @$required, @{ $optional || [] };
    for my $name (sort keys %given) {
        fail("$what has no argument `$name`") unless $known{$name};
    }
    for my $name (@$required) {
        fail("$what needs the argument `$name`") unless defined $given{$name};
    }
    return \%given;
}

sub instance {
    my ($value, $class, $name) = @_;
    my $short = (split /::/, $class)[-1];
    fail("$name must be a $short") unless Scalar::Util::blessed($value) && $value->isa($class);
    return $value;
}

1;
