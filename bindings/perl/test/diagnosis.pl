#!/usr/bin/env perl
# The diagnosis of a rejected send, read by the four SOAP results from a response the core wrote: the beta's real 2800
# answer (the recorded body in fe-kembec's cierre/datos/rechazado.json) run through the commands' exchange and diagnosis,
# kept in fixtures/send-fault-2800.json. Runnable alone (`perl -Ilib test/diagnosis.pl`, no library needed) and from
# vectors.pl, which calls run() with its own `check`.

package DiagnosisChecks;

use strict;
use warnings;

use File::Basename qw(dirname);
use File::Spec;
use JSON::PP ();

use Kembec::SunatSdk::Fields;
use Kembec::SunatSdk::Results;
use Kembec::SunatSdk::SoapResults;

my $FIXTURE = File::Spec->catfile(dirname(File::Spec->rel2abs(__FILE__)), 'fixtures', 'send-fault-2800.json');
my %KINDS = (send => 'SendResult', summary => 'SummaryResult', ticket => 'TicketResult', status => 'StatusResult');

# Calls $check->(ok, what) for each assertion.
sub run {
    my ($check) = @_;
    open my $in, '<:raw', $FIXTURE or die "$FIXTURE: $!";
    my $raw = do { local $/; <$in> };
    close $in;
    my $value = JSON::PP->new->utf8->decode($raw);
    for my $name (sort keys %KINDS) {
        my $class = "Kembec::SunatSdk::$KINDS{$name}";
        read_2800($check, $name, $class->from($value, $raw));
        my %without = %$value;
        delete $without{diagnosis};
        my $empty = $class->from(\%without, $raw);
        $check->(ref $empty->diagnosis eq 'ARRAY' && !@{ $empty->diagnosis }, "diagnosis $name: absent is an empty list");
    }
    $check->(Kembec::SunatSdk::SendResult->from($value, $raw)->fault->code eq '2800', 'diagnosis: the fault is still read');
    return;
}

sub read_2800 {
    my ($check, $name, $result) = @_;
    my $diagnosis = $result->diagnosis;
    $check->(@$diagnosis == 1, "diagnosis $name: " . @$diagnosis . ' elements');
    my $first = $diagnosis->[0] or return;
    $check->($first->code eq '2800', "diagnosis $name: code " . $first->code);
    my $detail = $first->detail;
    $check->($detail && $detail->node eq 'cbc:ID/schemeID' && $detail->value eq '1', "diagnosis $name: detail");
    my $field = $first->fields->[0];
    $check->(@{ $first->fields } == 1 && $field->input_path eq 'customer.document_type' && $field->certainty eq 'fuente'
        && $field->xpath =~ m{cbc:ID\@schemeID\z}, "diagnosis $name: field");
    return;
}

unless (caller) {
    my ($checks, $failures) = (0, 0);
    run(sub {
        my ($ok, $what) = @_;
        $checks++;
        return if $ok;
        $failures++;
        warn "FAIL $what\n";
    });
    print "ok Perl diagnosis: $checks checks\n" unless $failures;
    exit($failures ? 1 : 0);
}

1;
