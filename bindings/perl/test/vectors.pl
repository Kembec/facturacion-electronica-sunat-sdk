#!/usr/bin/env perl
# The battery of the module as a consumer installs it: the 38 fixtures against their vectors (inclusive C14N, exact
# DigestValue and SignatureValue), the native version, zip and CDR, the errors, no_verdict without an answer, and
# concurrent calls from forked children and from threads. Exit code 0 only if every check passed.
#
#   perl vectors.pl <repository>
#
# C14N needs XML::LibXML. SUNAT_SDK_FROM_SOURCE=1 accepts the module from -I instead of an installed one (development
# only); EXPECT_PLATFORM names the build the process must load ("linux-x64"...).

use strict;
use warnings;

use Config;
use Data::Dumper ();
use File::Basename qw(basename dirname);
use File::Spec;
use IO::Socket::INET;
use JSON::PP ();
use POSIX ();
use Time::HiRes ();
use XML::LibXML;

use Kembec::SunatSdk;

use constant EXPECTED_FIXTURES => 38;

my $repo = shift @ARGV;
die "usage: perl vectors.pl <repository>\n" if !defined $repo || @ARGV;
$repo = File::Spec->rel2abs($repo);

my ($checks, $failures) = (0, 0);
my $sdk = Kembec::SunatSdk->new(http_timeout => 30);
my $key = Kembec::SunatSdk::SigningKey->new(path(qw(tests vectors test-cert test-cert.pem)),
    path(qw(tests vectors test-cert test-key.pem)));
my @fixtures = sort glob(path('fixtures') . '/*.json');

check(@fixtures == EXPECTED_FIXTURES, 'fixtures: ' . @fixtures . ', expected ' . EXPECTED_FIXTURES);
step(version => \&version);
step(basename($_, '.json') => \&vector, $_) for @fixtures;
step(characters => \&characters);
step(zip => \&zip);
step(cdr => \&read_cdr);
step(errors => \&errors);
step(no_verdict => \&no_verdicts);
step(fork => \&forked);
step(threads => \&threaded);
exit report();

sub report {
    my $runtime = sprintf 'Perl %vd %s, %d-bit, FFI::Platypus %s', $^V, $Config{archname}, $Config{ptrsize} * 8,
        $FFI::Platypus::VERSION;
    if ($failures) {
        warn "Perl: $failures of $checks checks failed ($runtime)\n";
        return 1;
    }
    printf "ok Perl: %d checks, %d vectors, %s, C14N by XML::LibXML %s (libxml2 %s), native %s\n", $checks,
        scalar @fixtures, $runtime, $XML::LibXML::VERSION, XML::LibXML::LIBXML_DOTTED_VERSION(),
        Kembec::SunatSdk->native_version;
    return 0;
}

# The module that runs is the installed one, with its own library for this platform: never the source tree.
sub version {
    check(Kembec::SunatSdk->native_version eq Kembec::SunatSdk::SDK_VERSION,
        'native ' . Kembec::SunatSdk->native_version . ', module ' . Kembec::SunatSdk::SDK_VERSION);
    return if ($ENV{SUNAT_SDK_FROM_SOURCE} || '') eq '1';
    my $module = File::Spec->rel2abs($INC{'Kembec/SunatSdk.pm'});
    my $source = path(qw(bindings perl lib));
    check(index($module, $source) != 0, "the module was loaded from the source tree: $module");
    my $library = Kembec::SunatSdk::Native::library_path();
    my $installed = File::Spec->catdir(dirname($module), 'SunatSdk', 'native');
    check(index($library, $installed) == 0 && -f $library, "library $library is not the installed module's");
    my $expected = $ENV{EXPECT_PLATFORM} || '';
    my $platform = Kembec::SunatSdk::Platform::key();
    check($expected eq '' || $platform eq $expected, "platform $platform, expected $expected");
}

sub vector {
    my ($fixture) = @_;
    my $name = basename($fixture, '.json');
    my $document = slurp($fixture);
    my $built = $sdk->build($document);
    check($built->file_name eq expected_file_name(JSON::PP->new->utf8->decode($document)),
        "$name: file name " . $built->file_name);
    my $signed = $sdk->sign($built->xml, $key);
    my $expected = vector_text($name, 'signed');
    check(index($expected, '<ds:DigestValue>' . $signed->digest_value . '</ds:DigestValue>') >= 0,
        "$name: DigestValue " . $signed->digest_value);
    check(index($expected, '<ds:SignatureValue>' . $signed->signature_value . '</ds:SignatureValue>') >= 0,
        "$name: SignatureValue differs");
    check(index($signed->xml, '<ds:DigestValue>' . $signed->digest_value . '</ds:DigestValue>') >= 0,
        "$name: DigestValue not in the signed XML");
    compare($name, 'unsigned', $built->xml);
    compare($name, 'signed', $signed->xml);
}

sub compare {
    my ($name, $kind, $xml) = @_;
    check(canonical(vector_text($name, $kind)) eq canonical($xml), "$name: $kind XML differs from its vector under C14N");
}

# The same document as bytes read from the file, as Perl characters and as a hash reference signs the same; text in
# another encoding is refused, never misread.
sub characters {
    my $bytes = slurp(path(qw(fixtures 01-invoice-special-characters.json)));
    my $text = $bytes;
    utf8::decode($text);
    check(utf8::is_utf8($text) && length($text) < length($bytes), 'characters: the fixture has non-ASCII text');
    my $from_bytes = $sdk->sign($sdk->build($bytes)->xml, $key)->digest_value;
    check($sdk->sign($sdk->build($text)->xml, $key)->digest_value eq $from_bytes, 'characters: text and bytes differ');
    check($sdk->sign($sdk->build(JSON::PP->new->utf8->decode($bytes))->xml, $key)->digest_value eq $from_bytes,
        'characters: a hash reference signs differently');
    (my $latin1 = $bytes) =~ s/\xC3\xB1/\xF1/g;
    check($latin1 ne $bytes, 'characters: the fixture has an ñ');
    expect('Kembec::SunatSdk::ArgumentError', 'Latin-1 bytes', sub { $sdk->build($latin1) });
}

sub zip {
    my $document = JSON::PP->new->utf8->decode(slurp(path(qw(fixtures 01-invoice-taxed-cash.json))));
    my $signed = $sdk->sign($sdk->build($document)->xml, $key);
    check($signed->digest_value eq '+te61a2ub5MlTFf8tKTPNGWEXKI=', 'zip: a hash reference signs as its JSON text');
    my $zip = $sdk->zip('20123456789-01-F001-1', $signed->xml);
    check($zip->file_name eq '20123456789-01-F001-1.zip', 'zip: file name ' . $zip->file_name);
    check(!utf8::is_utf8($zip->content) && substr($zip->content, 0, 2) eq 'PK', 'zip: not a zip of bytes');
}

sub read_cdr {
    my $cdr = $sdk->read_cdr(slurp(path(qw(crates sunat-transport tests data R-20150147718-01-FT01-982.zip))));
    check($cdr->reference_id eq 'FT01-982', 'cdr: reference ' . $cdr->reference_id);
    check($cdr->response_code eq '2047', 'cdr: code ' . $cdr->response_code);
    check($cdr->response_class eq 'rejected' && !$cdr->is_valid_document, 'cdr: class ' . $cdr->response_class);
}

sub errors {
    my $error = expect('Kembec::SunatSdk::Error', 'build of {} is the SDK error', sub { $sdk->build('{}') });
    check(!$error || $error->command eq 'build', 'build of {}: command ' . ($error ? $error->command : ''));
    expect('Kembec::SunatSdk::ArgumentError', 'invalid JSON', sub { $sdk->build('{not json') });
    expect('Kembec::SunatSdk::ArgumentError', 'an undefined document', sub { $sdk->build(undef) });
    expect('Kembec::SunatSdk::ArgumentError', 'timeout of 1.5 s', sub { Kembec::SunatSdk->new(http_timeout => 1.5) });
    expect('Kembec::SunatSdk::ArgumentError', 'an unknown argument', sub { Kembec::SunatSdk->new(timeout => 5) });
    expect('Kembec::SunatSdk::ArgumentError', 'a wide character in a CDR', sub { $sdk->read_cdr("PK\x{263A}") });
    for my $total (118.0, 118.5) {
        expect('Kembec::SunatSdk::ArgumentError', "a floating-point total $total",
            sub { $sdk->validity(voucher(), issue_date => '2026-10-01', total => $total, credentials => api()) });
    }
    my $sol = Kembec::SunatSdk::SolCredentials->new('20123456789MODDATOS', 'moddatos-secret');
    my $shown = "$sol" . Data::Dumper->new([$sol, api()])->Deparse(0)->Dump;
    check(index($shown, 'moddatos-secret') < 0 && index($shown, 'client-secret') < 0, 'credentials show a secret');
    my $signed = $sdk->sign($sdk->build(slurp(path(qw(fixtures 01-invoice-taxed-cash.json))))->xml, $key)->xml;
    $error = expect('Kembec::SunatSdk::Error', 'send over http://', sub {
        $sdk->send('20123456789-01-F001-1', $signed, credentials => $sol, endpoint => 'http://127.0.0.1/billService');
    });
    check(!$error || ($error->command eq 'send' && index($error->message, 'endpoint must use https://') >= 0
            && index($error->message, 'moddatos-secret') < 0), 'send over http://: ' . ($error || ''));
}

# A listener that never accepts: the kernel completes the TCP handshake and nobody answers the TLS one, so every query
# waits out its 1 s.
sub no_verdicts {
    my $server = IO::Socket::INET->new(LocalAddr => '127.0.0.1', LocalPort => 0, Listen => 16, Proto => 'tcp')
        or die "listen: $!";
    my $base = 'https://127.0.0.1:' . $server->sockport;
    my $client = Kembec::SunatSdk->new(http_timeout => 1);
    my $sol = Kembec::SunatSdk::SolCredentials->new('20123456789MODDATOS', 'moddatos');
    my $started = Time::HiRes::time();
    my $status = $client->status(voucher(), credentials => $sol, endpoint => "$base/billConsultService");
    my $waited = Time::HiRes::time() - $started;
    check($status->outcome eq 'no_verdict' && $status->no_verdict->reason eq 'timeout'
            && !defined $status->no_verdict->http_status, 'status without an answer: ' . $status->raw_json);
    check($waited >= 0.9 && $waited < 10, "status without an answer waited $waited s for a 1 s timeout");
    my $validity = $client->validity(voucher(), issue_date => '2026-10-01', total => '118.00', credentials => api(),
        token_endpoint => "$base/token", endpoint => "$base/query");
    check($validity->outcome eq 'no_verdict' && $validity->no_verdict->stage eq 'token',
        'validity without a token: ' . $validity->raw_json);
    my $gre = $client->gre_ticket('1700000000000', api => api(), sol => $sol, token_endpoint => "$base/token",
        endpoint => "$base/v1");
    check($gre->outcome eq 'no_verdict' && $gre->no_verdict->stage eq 'token' && $gre->ticket eq '1700000000000',
        'gre-ticket without a token: ' . $gre->raw_json);
    close $server;
}

# Prefork servers (Starman, Apache) load the module and then fork: eight children sign different fixtures at once, after
# the parent already loaded the library, and each must still match its vector.
sub forked {
    if ($^O eq 'MSWin32') {
        check(1, 'fork: emulated by threads on Windows, covered by the threads check');
        return;
    }
    my @names = first_names(8);
    my %children;
    for my $name (@names) {
        pipe(my $reader, my $writer) or die "pipe: $!";
        my $pid = fork;
        die "fork: $!" unless defined $pid;
        if (!$pid) {
            close $reader;
            my $digest = eval { sign_five_times($name) } || 'died';
            print {$writer} $digest;
            close $writer;
            POSIX::_exit(0);
        }
        close $writer;
        $children{$name} = [$pid, $reader];
    }
    for my $name (@names) {
        my ($pid, $reader) = @{ $children{$name} };
        my $digest = do { local $/; <$reader> };
        waitpid $pid, 0;
        check($? == 0 && signed_with($name, $digest), "fork: $name gave " . ($digest // 'nothing'));
    }
}

sub threaded {
    unless ($Config{useithreads}) {
        check(1, 'threads: this perl has no ithreads');
        return;
    }
    require threads;
    my @names = first_names(8);
    my %workers = map { my $name = $_; ($name => threads->create(sub { sign_five_times($name) })) } @names;
    for my $name (@names) {
        my $digest = $workers{$name}->join;
        check(signed_with($name, $digest), "threads: $name gave " . ($digest // 'nothing'));
    }
}

sub sign_five_times {
    my ($name) = @_;
    my $document = slurp(path('fixtures', "$name.json"));
    my $digest;
    $digest = $sdk->sign($sdk->build($document)->xml, $key)->digest_value for 1 .. 5;
    return $digest;
}

sub signed_with {
    my ($name, $digest) = @_;
    return defined $digest && index(vector_text($name, 'signed'), "<ds:DigestValue>$digest</ds:DigestValue>") >= 0;
}

sub first_names { my ($count) = @_; return (map { basename($_, '.json') } @fixtures)[0 .. $count - 1] }

sub voucher { return (ruc => '20123456789', document_type => '01', series => 'F001', number => 1) }

sub api { Kembec::SunatSdk::ApiCredentials->new('client-id', 'client-secret') }

# ruc-type-series-number, or ruc-RC|RA-YYYYMMDD-number, from the fixture's own fields.
sub expected_file_name {
    my ($doc) = @_;
    my $ruc = $doc->{issuer}{ruc};
    return "$ruc-RC-" . ($doc->{generation_date} =~ tr/-//dr) . "-$doc->{number}" if $doc->{kind} eq 'summary';
    return "$ruc-RA-" . ($doc->{communication_date} =~ tr/-//dr) . "-$doc->{number}" if $doc->{kind} eq 'voided';
    return "$ruc-$doc->{document_type}-$doc->{series}-$doc->{number}";
}

# Inclusive C14N 1.0 without comments, from characters: the parser reads the UTF-8 bytes.
sub canonical {
    my ($xml) = @_;
    utf8::encode($xml) if utf8::is_utf8($xml);
    return XML::LibXML->new(no_network => 1, load_ext_dtd => 0)->parse_string($xml)->toStringC14N(0);
}

sub vector_text {
    my ($name, $kind) = @_;
    my $text = slurp(path('tests', 'vectors', "$name.$kind.xml"));
    utf8::decode($text);
    return $text;
}

sub slurp {
    my ($file) = @_;
    open my $handle, '<:raw', $file or die "$file: $!";
    local $/;
    return scalar <$handle>;
}

sub path { File::Spec->catfile($repo, @_) }

# Runs the code expecting exactly that error class; returns the error, or undef.
sub expect {
    my ($class, $what, $code) = @_;
    if (eval { $code->(); 1 }) {
        check(0, "$what: no error");
        return undef;
    }
    my $error = $@;
    check(ref $error eq $class, "$what: " . (ref $error || 'plain') . " $error");
    return ref $error eq $class ? $error : undef;
}

sub step {
    my ($what, $code, @arguments) = @_;
    eval { $code->(@arguments); 1 } or check(0, "$what: $@");
}

sub check {
    my ($ok, $what) = @_;
    $checks++;
    return if $ok;
    $failures++;
    warn "FAIL $what\n";
}
