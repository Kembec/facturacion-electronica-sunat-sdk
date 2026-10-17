package Kembec::SunatSdk::Result;

# What every result keeps: the response exactly as the library gave it. Results are read-only objects with one
# accessor per field; a field the response does not carry is undef.

use strict;
use warnings;

use JSON::PP ();

use Kembec::SunatSdk::Fields;

# Installs a read-only accessor per field in the calling class.
sub fields {
    my $class = caller;
    for my $name (@_) {
        no strict 'refs';
        *{"${class}::$name"} = sub { $_[0]->{$name} };
    }
    return;
}

sub new {
    my ($class, $raw_json, %fields) = @_;
    return bless { %fields, raw_json => $raw_json }, $class;
}

# The JSON of docs/cli.md for this result, for logs and for fields newer than this module.
sub raw_json { $_[0]->{raw_json} }

package Kembec::SunatSdk::BuildResult;

# build: the unsigned UBL XML and the SUNAT file name without extension.

use strict;
use warnings;

our @ISA = ('Kembec::SunatSdk::Result');

# file_name: "20123456789-01-F001-1", or "RUC-RC|RA-YYYYMMDD-N" for summaries and voids; xml: the unsigned XML, as
# Perl characters.
Kembec::SunatSdk::Result::fields(qw(file_name xml));

sub from {
    my ($class, $value, $raw) = @_;
    return $class->new($raw,
        file_name => Kembec::SunatSdk::Fields::text($value, 'file_name'),
        xml       => Kembec::SunatSdk::Fields::text($value, 'xml'));
}

package Kembec::SunatSdk::SignResult;

# sign: the signed XML and the two values of its signature.

use strict;
use warnings;

our @ISA = ('Kembec::SunatSdk::Result');

# digest_value: the DigestValue, base64, the hash printed on the voucher's representation; signature_value: the
# SignatureValue, base64.
Kembec::SunatSdk::Result::fields(qw(xml digest_value signature_value));

sub from {
    my ($class, $value, $raw) = @_;
    return $class->new($raw, map { $_ => Kembec::SunatSdk::Fields::text($value, $_) } qw(xml digest_value signature_value));
}

package Kembec::SunatSdk::ZipResult;

# zip: the zip SUNAT expects, with the XML inside under <name>.xml.

use strict;
use warnings;

our @ISA = ('Kembec::SunatSdk::Result');

# file_name: "20123456789-01-F001-1.zip"; content: the zip's bytes, a byte string.
Kembec::SunatSdk::Result::fields(qw(file_name content));

sub from {
    my ($class, $value, $raw) = @_;
    my $content = Kembec::SunatSdk::Fields::optional_base64($value, 'zip_base64');
    Kembec::SunatSdk::Fields::shape('`zip_base64` is missing') unless defined $content;
    return $class->new($raw, file_name => Kembec::SunatSdk::Fields::text($value, 'file_name'), content => $content);
}

package Kembec::SunatSdk::Cdr;

# A CDR (constancia de recepción): the code SUNAT answered and its class.

use strict;
use warnings;

our @ISA = ('Kembec::SunatSdk::Result');

# reference_id: the voucher it answers ("F001-1"); response_code: SUNAT's code ("0", "2047"...); notes: an array
# reference, each "code - text", with code 0 every note is an observation; response_class: how the SDK
# classifies the code and the notes, "accepted", "observed", "exception", "already_registered",
# "number_voided_or_rejected", "rejected", "void_already_reported" or "unknown".
Kembec::SunatSdk::Result::fields(qw(reference_id response_code description notes response_class));

sub from {
    my ($class, $value) = @_;
    return $class->new(JSON::PP->new->canonical->encode($value),
        reference_id   => Kembec::SunatSdk::Fields::text($value, 'reference_id'),
        response_code  => Kembec::SunatSdk::Fields::text($value, 'response_code'),
        description    => Kembec::SunatSdk::Fields::text($value, 'description'),
        notes          => Kembec::SunatSdk::Fields::texts($value, 'notes'),
        response_class => Kembec::SunatSdk::Fields::response_class($value));
}

# Code 0 without observations.
sub is_accepted { $_[0]->{response_class} eq 'accepted' ? 1 : 0 }

# Accepted, with or without observations: the voucher is valid.
sub is_valid_document { $_[0]->{response_class} =~ /\A(?:accepted|observed)\z/ ? 1 : 0 }

package Kembec::SunatSdk::Fault;

# A SOAP fault: SUNAT refused the call with a code.

use strict;
use warnings;

# code: SUNAT's code ("1033", "0111"...); response_class: as the Cdr's, "already_registered" is 1033.
Kembec::SunatSdk::Result::fields(qw(code message response_class));

sub from {
    my ($class, $value) = @_;
    return bless {
        code           => Kembec::SunatSdk::Fields::text($value, 'code'),
        message        => Kembec::SunatSdk::Fields::text($value, 'message'),
        response_class => Kembec::SunatSdk::Fields::response_class($value),
    }, $class;
}

package Kembec::SunatSdk::NoVerdict;

# No answer, or an HTTP status that says nothing about the document. It is not a rejection: ask again (docs/cli.md
# says when to resend the same signed XML with the same number).

use strict;
use warnings;

# reason: "timeout" or "http_status"; http_status: undef after a timeout; stage: for the REST exchanges, the call
# without an answer ("token" means nothing was sent), undef for SOAP.
Kembec::SunatSdk::Result::fields(qw(reason http_status message stage));

sub from {
    my ($class, $value) = @_;
    return bless {
        reason      => Kembec::SunatSdk::Fields::reason($value),
        http_status => Kembec::SunatSdk::Fields::optional_integer($value, 'http_status'),
        message     => Kembec::SunatSdk::Fields::text($value, 'message'),
        stage       => Kembec::SunatSdk::Fields::stage($value),
    }, $class;
}

1;
