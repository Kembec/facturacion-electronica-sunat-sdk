package Kembec::SunatSdk::ValidityResult;

# validity: the voucher's state in the Consulta Integrada ("validity"), SUNAT refusing the query ("query_error"), or
# "no_verdict".

use strict;
use warnings;

use Kembec::SunatSdk::Fields;
use Kembec::SunatSdk::Results;

our @ISA = ('Kembec::SunatSdk::Result');

# state: "not_reported" (0; it does not prove the voucher never arrived), "accepted" (1), "voided" (2),
# "printed_authorized" (3), "printed_not_authorized" (4) or "unknown"; observations: an array reference, where the
# data sent differs from SUNAT's record.
Kembec::SunatSdk::Result::fields(qw(outcome state_code state taxpayer_state_code domicile_condition_code observations
    message query_error no_verdict));

sub from {
    my ($class, $value, $raw) = @_;
    my $outcome = Kembec::SunatSdk::Fields::outcome($value, qw(validity query_error no_verdict));
    my %fields = (outcome => $outcome, state => 'unknown', observations => []);
    if ($outcome eq 'validity') {
        %fields = (%fields,
            state_code              => Kembec::SunatSdk::Fields::optional_text($value, 'state_code'),
            state                   => Kembec::SunatSdk::Fields::validity_state($value),
            taxpayer_state_code     => Kembec::SunatSdk::Fields::optional_text($value, 'taxpayer_state_code'),
            domicile_condition_code => Kembec::SunatSdk::Fields::optional_text($value, 'domicile_condition_code'),
            observations            => Kembec::SunatSdk::Fields::texts($value, 'observations'),
            message                 => Kembec::SunatSdk::Fields::optional_text($value, 'message'));
    }
    elsif ($outcome eq 'query_error') { $fields{query_error} = Kembec::SunatSdk::QueryError->from($value) }
    else { $fields{no_verdict} = Kembec::SunatSdk::NoVerdict->from($value) }
    return $class->new($raw, %fields);
}

package Kembec::SunatSdk::QueryError;

# SUNAT refused the Consulta Integrada query (about the query, not the voucher). error_code "99" is a malformed query.

use strict;
use warnings;

Kembec::SunatSdk::Result::fields(qw(http_status error_code message));

sub from {
    my ($class, $value) = @_;
    return bless {
        http_status => Kembec::SunatSdk::Fields::integer($value, 'http_status'),
        error_code  => Kembec::SunatSdk::Fields::optional_text($value, 'error_code'),
        message     => Kembec::SunatSdk::Fields::text($value, 'message'),
    }, $class;
}

package Kembec::SunatSdk::GreSendResult;

# gre_send: SUNAT received the guide and gave a ticket ("ticket"), a GRE API error ("api_error"), or "no_verdict" of
# the token (nothing was sent).

use strict;
use warnings;

our @ISA = ('Kembec::SunatSdk::Result');

# ticket: the only handle on the outcome, store it before anything else; hash_zip: SHA-256 of the zip, lowercase hex.
Kembec::SunatSdk::Result::fields(qw(outcome ticket received_at zip_name hash_zip api_error no_verdict));

sub from {
    my ($class, $value, $raw) = @_;
    my $outcome = Kembec::SunatSdk::Fields::outcome($value, qw(ticket api_error no_verdict));
    my %fields = (outcome => $outcome);
    if ($outcome eq 'ticket') {
        $fields{ticket} = Kembec::SunatSdk::Fields::text($value, 'ticket');
        $fields{$_} = Kembec::SunatSdk::Fields::optional_text($value, $_) for qw(received_at zip_name hash_zip);
    }
    elsif ($outcome eq 'api_error') { $fields{api_error} = Kembec::SunatSdk::GreApiError->from($value) }
    else { $fields{no_verdict} = Kembec::SunatSdk::NoVerdict->from($value) }
    return $class->new($raw, %fields);
}

package Kembec::SunatSdk::GreTicketResult;

# gre_ticket: the state of a sent guide ("ticket_status"), a GRE API error, or "no_verdict".

use strict;
use warnings;

our @ISA = ('Kembec::SunatSdk::Result');

# state: "processed" (0, with its cdr), "in_process" (98: ask again with the same ticket), "processed_with_errors" (99,
# with its error) or "unknown"; error: a Kembec::SunatSdk::GreTicketError; cdr_generated: whether SUNAT generated the
# guide's CDR, 1 or 0.
Kembec::SunatSdk::Result::fields(qw(outcome ticket status_code state error cdr_generated cdr cdr_zip api_error
    no_verdict));

sub from {
    my ($class, $value, $raw) = @_;
    my $outcome = Kembec::SunatSdk::Fields::outcome($value, qw(ticket_status api_error no_verdict));
    my %fields = (outcome => $outcome, ticket => Kembec::SunatSdk::Fields::text($value, 'ticket'),
        state => 'unknown', cdr_generated => 0);
    if ($outcome eq 'ticket_status') {
        my $error = Kembec::SunatSdk::Fields::optional_object($value, 'error');
        my $cdr = Kembec::SunatSdk::Fields::optional_object($value, 'cdr');
        %fields = (%fields,
            status_code   => Kembec::SunatSdk::Fields::text($value, 'status_code'),
            state         => Kembec::SunatSdk::Fields::ticket_state($value),
            cdr_generated => Kembec::SunatSdk::Fields::boolean($value, 'cdr_generated'),
            error         => $error && Kembec::SunatSdk::GreTicketError->from($error),
            cdr           => $cdr && Kembec::SunatSdk::Cdr->from($cdr),
            cdr_zip       => Kembec::SunatSdk::Fields::optional_base64($value, 'cdr_zip_base64'));
    }
    elsif ($outcome eq 'api_error') { $fields{api_error} = Kembec::SunatSdk::GreApiError->from($value) }
    else { $fields{no_verdict} = Kembec::SunatSdk::NoVerdict->from($value) }
    return $class->new($raw, %fields);
}

package Kembec::SunatSdk::GreTicketError;

# The error of a guide processed with errors (numError, desError).

use strict;
use warnings;

Kembec::SunatSdk::Result::fields(qw(code description response_class));

sub from {
    my ($class, $value) = @_;
    return bless {
        code           => Kembec::SunatSdk::Fields::optional_text($value, 'code'),
        description    => Kembec::SunatSdk::Fields::text($value, 'description'),
        response_class => Kembec::SunatSdk::Fields::response_class($value),
    }, $class;
}

package Kembec::SunatSdk::GreApiError;

# An error body of the GRE REST API (annex I of SUNAT's GRE manual).

use strict;
use warnings;

# stage: "token", "send" or "ticket"; general_error: annex I's status name ("unauthorized"...); errors: an array
# reference of the validation errors (a 422 lists them), each a hash reference with code and description.
Kembec::SunatSdk::Result::fields(qw(stage http_status general_error code message errors));

sub from {
    my ($class, $value) = @_;
    my $list = ref $value->{errors} eq 'ARRAY' ? $value->{errors} : [];
    my @errors = map {
        Kembec::SunatSdk::Fields::shape('an item of `errors` is not an object') unless ref $_ eq 'HASH';
        +{ code => Kembec::SunatSdk::Fields::optional_text($_, 'code'),
            description => Kembec::SunatSdk::Fields::text($_, 'description') }
    } @$list;
    return bless {
        stage         => Kembec::SunatSdk::Fields::stage($value),
        http_status   => Kembec::SunatSdk::Fields::integer($value, 'http_status'),
        general_error => Kembec::SunatSdk::Fields::optional_text($value, 'general_error'),
        code          => Kembec::SunatSdk::Fields::optional_text($value, 'code'),
        message       => Kembec::SunatSdk::Fields::text($value, 'message'),
        errors        => \@errors,
    }, $class;
}

1;
