package Kembec::SunatSdk::Diagnosis;

# A code SUNAT answered, with the input fields that may have caused it (the `diagnosis` of docs/cli.md).

use strict;
use warnings;

use Kembec::SunatSdk::Fields;
use Kembec::SunatSdk::Results;

# code: SUNAT's code ("2800", "4287"...); detail: a Kembec::SunatSdk::DiagnosisDetail with what SUNAT's message names,
# or undef when it names none; fields: a list of Kembec::SunatSdk::DiagnosisField, empty for a code the rules do not
# list and for summaries, void communications and tickets (with a detail, only those on its node, unless none is).
Kembec::SunatSdk::Result::fields(qw(code detail fields));

sub from {
    my ($class, $value) = @_;
    my $detail = Kembec::SunatSdk::Fields::optional_object($value, 'detail');
    return bless {
        code   => Kembec::SunatSdk::Fields::text($value, 'code'),
        detail => $detail && Kembec::SunatSdk::DiagnosisDetail->from($detail),
        fields => [map { Kembec::SunatSdk::DiagnosisField->from($_) } @{ Kembec::SunatSdk::Fields::objects($value, 'fields') }],
    }, $class;
}

# The `diagnosis` of an outcome: one per code in the answer (the fault's, a CDR's non-zero response_code, each note that
# starts with a code); an empty list when the output has none.
sub list_from {
    my ($class, $value) = @_;
    return [map { $class->from($_) } @{ Kembec::SunatSdk::Fields::objects($value, 'diagnosis') }];
}

package Kembec::SunatSdk::DiagnosisDetail;

# What SUNAT's message names as (nodo: "<node>" valor: "<value>"), e.g. "cbc:ID/schemeID" = "1". The form is observed in
# beta answers; SUNAT does not document it.

use strict;
use warnings;

# node: the last element and, if any, its attribute, as SUNAT writes it; value: what SUNAT read in that node.
Kembec::SunatSdk::Result::fields(qw(node value));

sub from {
    my ($class, $value) = @_;
    return bless {
        node  => Kembec::SunatSdk::Fields::text($value, 'node'),
        value => Kembec::SunatSdk::Fields::text($value, 'value'),
    }, $class;
}

package Kembec::SunatSdk::DiagnosisField;

# One candidate field for a code, from SUNAT's validation rules of 26.08.2026 (docs/cli.md#diagnosis).

use strict;
use warnings;

# input_path: a field of docs/input-model.md ("customer.document_type"), "sdk" when the SDK writes the node, or undef
# when the input model has no such field; xpath: the node the rule checks, undef for rules on the file, the signature or
# the service; certainty: "fuente" when the node is the rule's own and one field writes it, "inferido" when the rule
# computes over several nodes.
Kembec::SunatSdk::Result::fields(qw(input_path xpath certainty));

sub from {
    my ($class, $value) = @_;
    return bless {
        input_path => Kembec::SunatSdk::Fields::optional_text($value, 'input_path'),
        xpath      => Kembec::SunatSdk::Fields::optional_text($value, 'xpath'),
        certainty  => Kembec::SunatSdk::Fields::text($value, 'certainty'),
    }, $class;
}

1;
