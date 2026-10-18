package Kembec::SunatSdk::SendResult;

# send: SUNAT's CDR (outcome "cdr") or a fault (outcome "fault").

use strict;
use warnings;

use Kembec::SunatSdk::Diagnosis;
use Kembec::SunatSdk::Fields;
use Kembec::SunatSdk::Results;

our @ISA = ('Kembec::SunatSdk::Result');

# cdr: a Kembec::SunatSdk::Cdr, with outcome "cdr"; cdr_zip: the CDR zip as SUNAT sent it, a byte string, keep it;
# fault: a Kembec::SunatSdk::Fault, with outcome "fault".
Kembec::SunatSdk::Result::fields(qw(outcome cdr cdr_zip fault diagnosis));

sub from {
    my ($class, $value, $raw) = @_;
    my $outcome = Kembec::SunatSdk::Fields::outcome($value, qw(cdr fault));
    my $diagnosis = Kembec::SunatSdk::Diagnosis->list_from($value);
    return $class->new($raw, outcome => $outcome, fault => Kembec::SunatSdk::Fault->from($value),
        diagnosis => $diagnosis) if $outcome eq 'fault';
    my $cdr = Kembec::SunatSdk::Fields::optional_object($value, 'cdr');
    Kembec::SunatSdk::Fields::shape('`cdr` is missing') unless $cdr;
    return $class->new($raw,
        outcome => $outcome,
        cdr     => Kembec::SunatSdk::Cdr->from($cdr),
        cdr_zip => Kembec::SunatSdk::Fields::optional_base64($value, 'cdr_zip_base64'),
        diagnosis => $diagnosis);
}

package Kembec::SunatSdk::SummaryResult;

# summary: SUNAT received the summary or void and gave a ticket (outcome "ticket"), or a fault.

use strict;
use warnings;

our @ISA = ('Kembec::SunatSdk::Result');

# ticket: the only handle on the outcome, store it before anything else.
Kembec::SunatSdk::Result::fields(qw(outcome ticket fault diagnosis));

sub from {
    my ($class, $value, $raw) = @_;
    my $outcome = Kembec::SunatSdk::Fields::outcome($value, qw(ticket fault));
    my $diagnosis = Kembec::SunatSdk::Diagnosis->list_from($value);
    return $class->new($raw, outcome => $outcome, fault => Kembec::SunatSdk::Fault->from($value),
        diagnosis => $diagnosis) if $outcome eq 'fault';
    return $class->new($raw, outcome => $outcome, ticket => Kembec::SunatSdk::Fields::text($value, 'ticket'),
        diagnosis => $diagnosis);
}

package Kembec::SunatSdk::TicketResult;

# ticket: the state of a summary's or a void's ticket ("ticket_status"), a fault, or "no_verdict".

use strict;
use warnings;

our @ISA = ('Kembec::SunatSdk::Result');

# state: "processed" (0), "in_process" (98: ask again with the same ticket, never send again), "processed_with_errors"
# (99) or "unknown".
Kembec::SunatSdk::Result::fields(qw(outcome ticket status_code state cdr cdr_zip fault no_verdict diagnosis));

sub from {
    my ($class, $value, $raw) = @_;
    my $outcome = Kembec::SunatSdk::Fields::outcome($value, qw(ticket_status fault no_verdict));
    my %fields = (outcome => $outcome, ticket => Kembec::SunatSdk::Fields::optional_text($value, 'ticket'),
        state => 'unknown', diagnosis => Kembec::SunatSdk::Diagnosis->list_from($value));
    if ($outcome eq 'ticket_status') {
        my $cdr = Kembec::SunatSdk::Fields::optional_object($value, 'cdr');
        %fields = (%fields,
            status_code => Kembec::SunatSdk::Fields::text($value, 'status_code'),
            state       => Kembec::SunatSdk::Fields::ticket_state($value),
            cdr         => $cdr && Kembec::SunatSdk::Cdr->from($cdr),
            cdr_zip     => Kembec::SunatSdk::Fields::optional_base64($value, 'cdr_zip_base64'));
    }
    elsif ($outcome eq 'fault') { $fields{fault} = Kembec::SunatSdk::Fault->from($value) }
    else { $fields{no_verdict} = Kembec::SunatSdk::NoVerdict->from($value) }
    return $class->new($raw, %fields);
}

package Kembec::SunatSdk::StatusResult;

# status: getStatusCdr's answer ("status"), a fault, or "no_verdict".

use strict;
use warnings;

our @ISA = ('Kembec::SunatSdk::Result');

# cdr and cdr_zip: when SUNAT attaches the CDR.
Kembec::SunatSdk::Result::fields(qw(outcome status_code status_message cdr cdr_zip fault no_verdict diagnosis));

sub from {
    my ($class, $value, $raw) = @_;
    my $outcome = Kembec::SunatSdk::Fields::outcome($value, qw(status fault no_verdict));
    my %fields = (outcome => $outcome, diagnosis => Kembec::SunatSdk::Diagnosis->list_from($value));
    if ($outcome eq 'status') {
        my $cdr = Kembec::SunatSdk::Fields::optional_object($value, 'cdr');
        %fields = (%fields,
            status_code    => Kembec::SunatSdk::Fields::text($value, 'status_code'),
            status_message => Kembec::SunatSdk::Fields::optional_text($value, 'status_message'),
            cdr            => $cdr && Kembec::SunatSdk::Cdr->from($cdr),
            cdr_zip        => Kembec::SunatSdk::Fields::optional_base64($value, 'cdr_zip_base64'));
    }
    elsif ($outcome eq 'fault') { $fields{fault} = Kembec::SunatSdk::Fault->from($value) }
    else { $fields{no_verdict} = Kembec::SunatSdk::NoVerdict->from($value) }
    return $class->new($raw, %fields);
}

1;
