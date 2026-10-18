package Kembec::SunatSdk::Fields;

# Reads the fields of a response (docs/cli.md). A field that is missing or of another type dies with a ShapeError,
# which the client reports as Kembec::SunatSdk::Error. Names of outcomes, classes and states stay the contract's
# strings ("cdr", "no_verdict", "accepted"...).

use strict;
use warnings;

use MIME::Base64 ();

my %CLASSES = map { $_ => 1 }
    qw(accepted observed exception already_registered number_voided_or_rejected rejected void_already_reported);
my %TICKET_STATES = map { $_ => 1 } qw(processed in_process processed_with_errors);
my %VALIDITY_STATES = map { $_ => 1 } qw(not_reported accepted voided printed_authorized printed_not_authorized);
my %STAGES = map { $_ => 1 } qw(token query send ticket);
my %REASONS = map { $_ => 1 } qw(timeout http_status);

sub shape { die bless { message => $_[0] }, 'Kembec::SunatSdk::ShapeError' }

sub text {
    my ($value, $name) = @_;
    my $field = optional_text($value, $name);
    shape("`$name` is missing") unless defined $field;
    return $field;
}

sub optional_text {
    my ($value, $name) = @_;
    my $field = $value->{$name};
    return undef unless defined $field;
    shape("`$name` is not a string") if ref $field;
    return $field;
}

sub integer {
    my ($value, $name) = @_;
    my $field = optional_integer($value, $name);
    shape("`$name` is missing") unless defined $field;
    return $field;
}

sub optional_integer {
    my ($value, $name) = @_;
    my $field = $value->{$name};
    return undef unless defined $field;
    shape("`$name` is not a whole number") if ref $field || $field !~ /\A-?[0-9]+\z/;
    return 0 + $field;
}

sub boolean {
    my ($value, $name) = @_;
    my $field = $value->{$name};
    shape("`$name` is not a boolean") unless ref $field && $field->isa('JSON::PP::Boolean');
    return $field ? 1 : 0;
}

sub texts {
    my ($value, $name) = @_;
    my $field = $value->{$name};
    return [] unless defined $field;
    shape("`$name` is not a list of strings") unless ref $field eq 'ARRAY' && !grep { !defined || ref } @$field;
    return [@$field];
}

# The objects of a list; an absent list is empty.
sub objects {
    my ($value, $name) = @_;
    my $field = $value->{$name};
    return [] unless defined $field;
    shape("`$name` is not a list of objects") unless ref $field eq 'ARRAY' && !grep { ref ne 'HASH' } @$field;
    return $field;
}

sub optional_object {
    my ($value, $name) = @_;
    my $field = $value->{$name};
    return undef unless defined $field;
    shape("`$name` is not an object") unless ref $field eq 'HASH';
    return $field;
}

# The decoded bytes of a base64 field, as a byte string.
sub optional_base64 {
    my ($value, $name) = @_;
    my $encoded = optional_text($value, $name);
    return undef unless defined $encoded;
    shape("`$name` is not base64") unless $encoded =~ m{\A[A-Za-z0-9+/]*={0,2}\z} && length($encoded) % 4 == 0;
    utf8::downgrade($encoded);
    return MIME::Base64::decode_base64($encoded);
}

# The outcome, one of those the command gives.
sub outcome {
    my ($value, @allowed) = @_;
    my $name = text($value, 'outcome');
    shape("outcome `$name` is not one this command gives") unless grep { $_ eq $name } @allowed;
    return $name;
}

sub response_class { _known(\%CLASSES, optional_text($_[0], 'class')) }
sub ticket_state   { _known(\%TICKET_STATES, optional_text($_[0], 'state')) }
sub validity_state { _known(\%VALIDITY_STATES, optional_text($_[0], 'state')) }

# undef for a SOAP exchange, which has a single call.
sub stage {
    my ($value) = @_;
    my $name = optional_text($value, 'stage');
    return undef unless defined $name;
    shape("unknown stage `$name`") unless $STAGES{$name};
    return $name;
}

sub reason {
    my ($value) = @_;
    my $name = text($value, 'reason');
    shape("unknown no_verdict reason `$name`") unless $REASONS{$name};
    return $name;
}

sub _known {
    my ($names, $name) = @_;
    return defined $name && $names->{$name} ? $name : 'unknown';
}

1;
