use strict;
use warnings;

use File::Spec;
use Test::More;

use Kembec::SunatSdk;

# A fixture of the repository, copied into t/data by scripts/build-dist.sh: the build and the errors as the module
# gives them. Signing and the 38 vectors run from the repository (test/vectors.pl), where the test certificate lives.
my $fixture = File::Spec->catfile('t', 'data', '01-invoice-taxed-cash.json');
plan skip_all => "$fixture is staged by scripts/build-dist.sh" unless -f $fixture;

my $sdk = Kembec::SunatSdk->new;
my $document = do { open my $handle, '<:raw', $fixture or die "$fixture: $!"; local $/; <$handle> };
my $built = $sdk->build($document);
is($built->file_name, '20123456789-01-F001-1', 'file name');
like($built->xml, qr{<cbc:ID>F001-1</cbc:ID>}, 'the XML carries the voucher');
like($built->raw_json, qr/"file_name"/, 'raw_json');

my $error = eval { $sdk->build('{}'); 1 } ? undef : $@;
isa_ok($error, 'Kembec::SunatSdk::Error', 'an invalid document');
is($error && $error->command, 'build', 'the error names its command');

$error = eval { $sdk->build('{not json'); 1 } ? undef : $@;
isa_ok($error, 'Kembec::SunatSdk::ArgumentError', 'text that is not JSON');

$error = eval { $sdk->status(ruc => '20123456789'); 1 } ? undef : $@;
isa_ok($error, 'Kembec::SunatSdk::ArgumentError', 'a missing named argument');

done_testing;
