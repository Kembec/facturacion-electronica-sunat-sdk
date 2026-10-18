package Kembec::SunatSdk;

# The commands of the SDK's contract (docs/cli.md) in process, through its C ABI (docs/c-abi.md): the same validation,
# XML, signature, network calls and outcomes as the CLI and every other language. The object only holds the HTTP wait;
# it keeps no state between calls and can be shared by forked children and threads.

use strict;
use warnings;

# CPAN orders versions as decimals: X.Y.Z is X.00Y00Z, and beta N of X.Y.Z is the decimal just below it with _N, a
# trial that sorts before the final release (0.1.0-beta.8 is 0.000999_008, 0.1.0 will be 0.001000).
our $VERSION = '0.000999_008';
$VERSION =~ tr/_//d;

use JSON::PP ();
use MIME::Base64 ();
use Scalar::Util ();

use Kembec::SunatSdk::Arguments;
use Kembec::SunatSdk::Credentials;
use Kembec::SunatSdk::Diagnosis;
use Kembec::SunatSdk::Error;
use Kembec::SunatSdk::Fields;
use Kembec::SunatSdk::Native;
use Kembec::SunatSdk::Results;
use Kembec::SunatSdk::RestResults;
use Kembec::SunatSdk::SoapResults;

# The SDK's version (Cargo.toml), as the native library reports it.
use constant SDK_VERSION => '0.1.0-beta.8';

# The C ABI this module speaks; a library with another one is refused at load.
use constant ABI_VERSION => Kembec::SunatSdk::Native::ABI_VERSION;

# 90 seconds, the CLI's default; at most 300.
use constant DEFAULT_HTTP_TIMEOUT => 90;
use constant MAX_HTTP_TIMEOUT     => 300;

my $JSON = JSON::PP->new->utf8->canonical;

# http_timeout: how long a network command waits on SUNAT, whole seconds from 1 to 300; the token and the call of the
# REST commands share it.
sub new {
    my $class = shift;
    my $arguments = Kembec::SunatSdk::Arguments::named('new', \@_, [], ['http_timeout']);
    my $timeout = $arguments->{http_timeout} // DEFAULT_HTTP_TIMEOUT;
    Kembec::SunatSdk::Arguments::fail('http_timeout must be whole seconds from 1 to ' . MAX_HTTP_TIMEOUT . ", not $timeout")
        if ref $timeout || "$timeout" !~ /\A[0-9]+\z/ || $timeout < 1 || $timeout > MAX_HTTP_TIMEOUT;
    return bless { http_timeout => 0 + $timeout }, $class;
}

sub http_timeout { $_[0]->{http_timeout} }

# The SDK version of the native library; it loads the library.
sub native_version { Kembec::SunatSdk::Native::version() }

# Renders the unsigned UBL XML of one voucher, daily summary, void communication or despatch advice. $document: the
# JSON text of docs/input-model.md, or a hash reference in that shape (amounts as strings).
sub build {
    my ($self, $document) = @_;
    return _run('build', _document_json($document), undef, 'Kembec::SunatSdk::BuildResult');
}

# Signs the XML of build with XML-DSig (enveloped, RSA-SHA1, as SUNAT requires); signature_id is "signatureKG" when
# not given.
sub sign {
    my ($self, $unsigned_xml, $key, @options) = @_;
    my $options = Kembec::SunatSdk::Arguments::named('sign', \@options, [], ['signature_id']);
    Kembec::SunatSdk::Arguments::instance($key, 'Kembec::SunatSdk::SigningKey', 'key');
    my $input = _object(xml => _text($unsigned_xml, 'unsigned_xml'),
        signature_id => Kembec::SunatSdk::Arguments::optional($options->{signature_id}, 'signature_id'));
    my $flags = { cert => $key->certificate_path, key => $key->private_key_path };
    return _run('sign', $input, $flags, 'Kembec::SunatSdk::SignResult');
}

# Zips the XML under <name>.xml, as SUNAT expects it. $name: the file name without extension.
sub zip {
    my ($self, $name, $xml) = @_;
    return _run('zip', _object(name => _text($name, 'name'), xml => _text($xml, 'xml')), undef,
        'Kembec::SunatSdk::ZipResult');
}

# Reads a CDR, the bytes of its zip or its XML, and classifies its response code (the cdr command).
sub read_cdr {
    my ($self, $cdr) = @_;
    Kembec::SunatSdk::Arguments::fail('cdr is required') unless defined $cdr;
    Kembec::SunatSdk::Arguments::fail('cdr must be a byte string') if ref $cdr;
    my $bytes = $cdr;
    Kembec::SunatSdk::Arguments::fail('cdr must be a byte string, not text with wide characters')
        unless utf8::downgrade($bytes, 1);
    my $input = _object(cdr_base64 => MIME::Base64::encode_base64($bytes, ''));
    my ($value) = Kembec::SunatSdk::Native::run('cdr', $input);
    return _shaped('cdr', sub { Kembec::SunatSdk::Cdr->from($value) });
}

# Zips the signed XML and sends it with sendBill. A fault is a result; a send without an answer dies with Error,
# since it may have arrived: send the same XML again, never another number.
sub send {
    my ($self, $name, $signed_xml, @rest) = @_;
    my $arguments = Kembec::SunatSdk::Arguments::named('send', \@rest, [qw(credentials endpoint)]);
    my $input = _object(name => _text($name, 'name'), xml => _text($signed_xml, 'signed_xml'));
    return _run('send', $input, $self->_soap($arguments), 'Kembec::SunatSdk::SendResult');
}

# Zips a signed daily summary (RC) or void communication (RA) and sends it with sendSummary. Store the ticket before
# anything else. A send without an answer dies, as send.
sub summary {
    my ($self, $name, $signed_xml, @rest) = @_;
    my $arguments = Kembec::SunatSdk::Arguments::named('summary', \@rest, [qw(credentials endpoint)]);
    my $input = _object(name => _text($name, 'name'), xml => _text($signed_xml, 'signed_xml'));
    return _run('summary', $input, $self->_soap($arguments), 'Kembec::SunatSdk::SummaryResult');
}

# Asks getStatus for a summary's or a void's ticket. No answer is "no_verdict": ask again, never send again.
sub ticket {
    my ($self, $ticket, @rest) = @_;
    my $arguments = Kembec::SunatSdk::Arguments::named('ticket', \@rest, [qw(credentials endpoint)]);
    return _run('ticket', _object(ticket => _text($ticket, 'ticket')), $self->_soap($arguments),
        'Kembec::SunatSdk::TicketResult');
}

# Asks getStatusCdr for one voucher (01, 03, 07, 08), with its CDR when SUNAT attaches it. No answer is "no_verdict".
# endpoint: the billConsultService URL, https only.
sub status {
    my $self = shift;
    my $arguments = Kembec::SunatSdk::Arguments::named('status', \@_,
        [qw(ruc document_type series number credentials endpoint)]);
    return _run('status', _object(_voucher($arguments)), $self->_soap($arguments), 'Kembec::SunatSdk::StatusResult');
}

# Asks SUNAT's Consulta Integrada whether a voucher (01, 03, 07, 08) exists and in which state: a token, then the query,
# both inside http_timeout. There is no test environment: this is production. issue_date: "YYYY-MM-DD", or a DateTime
# or Time::Piece (its own calendar date); total: the PayableAmount as a string or a Math::BigFloat, up to 2 decimals, never a float;
# credentials: the Consulta Integrada ApiCredentials; requester_ruc: the issuer's when not given; token_endpoint and
# endpoint replace the production bases (https only).
sub validity {
    my $self = shift;
    my $arguments = Kembec::SunatSdk::Arguments::named('validity', \@_,
        [qw(ruc document_type series number issue_date total credentials)],
        [qw(requester_ruc token_endpoint endpoint)]);
    my %fields = (_voucher($arguments),
        issue_date    => _date($arguments->{issue_date}),
        total         => Kembec::SunatSdk::Arguments::amount($arguments->{total}, 'total'),
        requester_ruc => Kembec::SunatSdk::Arguments::optional($arguments->{requester_ruc}, 'requester_ruc'));
    my $credentials
        = Kembec::SunatSdk::Arguments::instance($arguments->{credentials}, 'Kembec::SunatSdk::ApiCredentials', 'credentials');
    my $flags = $credentials->_flags;
    for my $name (qw(token_endpoint endpoint)) {
        my $value = Kembec::SunatSdk::Arguments::optional($arguments->{$name}, $name);
        (my $flag = $name) =~ tr/_/-/;
        $flags->{$flag} = $value if defined $value;
    }
    $flags->{timeout} = $self->{http_timeout};
    return _run('validity', _object(%fields), $flags, 'Kembec::SunatSdk::ValidityResult');
}

# Sends a signed despatch advice (09) by SUNAT's GRE REST API: a token by password, then the zip with its hash. SUNAT
# has no test environment for GRE and the endpoints have no default. A send without an answer dies: send the same file
# again, never the operation with another number. api: the GRE ApiCredentials; sol: the SolCredentials;
# token_endpoint and endpoint: https bases.
sub gre_send {
    my ($self, $name, $signed_xml, @rest) = @_;
    my $arguments = Kembec::SunatSdk::Arguments::named('gre_send', \@rest, [qw(api sol token_endpoint endpoint)]);
    my $input = _object(name => _text($name, 'name'), xml => _text($signed_xml, 'signed_xml'));
    return _run('gre-send', $input, $self->_gre($arguments), 'Kembec::SunatSdk::GreSendResult');
}

# Asks for the outcome of a sent guide by its ticket (a token, then the query). "in_process" (98): ask again with the
# same ticket.
sub gre_ticket {
    my ($self, $ticket, @rest) = @_;
    my $arguments = Kembec::SunatSdk::Arguments::named('gre_ticket', \@rest, [qw(api sol token_endpoint endpoint)]);
    return _run('gre-ticket', _object(ticket => _text($ticket, 'ticket')), $self->_gre($arguments),
        'Kembec::SunatSdk::GreTicketResult');
}

sub _soap {
    my ($self, $arguments) = @_;
    my $sol = Kembec::SunatSdk::Arguments::instance($arguments->{credentials}, 'Kembec::SunatSdk::SolCredentials',
        'credentials');
    return { %{ $sol->_flags }, endpoint => _text($arguments->{endpoint}, 'endpoint'),
        timeout => $self->{http_timeout} };
}

sub _gre {
    my ($self, $arguments) = @_;
    my $api = Kembec::SunatSdk::Arguments::instance($arguments->{api}, 'Kembec::SunatSdk::ApiCredentials', 'api');
    my $sol = Kembec::SunatSdk::Arguments::instance($arguments->{sol}, 'Kembec::SunatSdk::SolCredentials', 'sol');
    return { %{ $api->_flags }, %{ $sol->_flags },
        'token-endpoint' => _text($arguments->{token_endpoint}, 'token_endpoint'),
        endpoint         => _text($arguments->{endpoint}, 'endpoint'),
        timeout          => $self->{http_timeout} };
}

sub _voucher {
    my ($arguments) = @_;
    return (
        ruc           => _text($arguments->{ruc}, 'ruc'),
        document_type => _text($arguments->{document_type}, 'document_type'),
        series        => _text($arguments->{series}, 'series'),
        number        => Kembec::SunatSdk::Arguments::whole($arguments->{number}, 'number'),
    );
}

sub _run {
    my ($command, $input, $flags, $class) = @_;
    my ($value, $raw) = Kembec::SunatSdk::Native::run($command, $input, $flags);
    return _shaped($command, sub { $class->from($value, $raw) });
}

# A response that is not the contract's is the SDK's error, with the field that broke it.
sub _shaped {
    my ($command, $read) = @_;
    my $result = eval { $read->() };
    return $result if $result;
    my $error = $@;
    die $error unless ref $error eq 'Kembec::SunatSdk::ShapeError';
    Kembec::SunatSdk::Error->throw("unexpected response: $error->{message}", command => $command);
}

# The document's JSON text exactly as the caller wrote it, after checking it is one JSON value: amounts are decimal
# strings and must reach the SDK untouched.
sub _document_json {
    my ($document) = @_;
    return $JSON->encode($document) if ref $document eq 'HASH';
    my $text = Kembec::SunatSdk::Arguments::characters($document, 'document');
    eval { JSON::PP->new->allow_nonref->decode($text); 1 }
        or Kembec::SunatSdk::Arguments::fail('document is not one JSON value: ' . ($@ =~ s/ at \S+ line \d+.*//sr));
    utf8::encode($text);
    return $text;
}

sub _object {
    my %fields = @_;
    return $JSON->encode({ map { defined $fields{$_} ? ($_ => $fields{$_}) : () } keys %fields });
}

sub _text { Kembec::SunatSdk::Arguments::characters(@_) }

# DateTime and Time::Piece give their calendar date with ymd; their default text is not "YYYY-MM-DD".
sub _date {
    my ($date) = @_;
    return $date->ymd if Scalar::Util::blessed($date) && $date->can('ymd');
    return _text($date, 'issue_date');
}

1;

__END__

=encoding utf8

=head1 NAME

Kembec::SunatSdk - SUNAT (Peru) electronic invoicing SDK: build, sign and send vouchers from Perl

=head1 SYNOPSIS

    use Kembec::SunatSdk;

    my $sdk    = Kembec::SunatSdk->new(http_timeout => 60);
    my $built  = $sdk->build($document_json);     # el JSON o un hash de docs/input-model.md
    my $signed = $sdk->sign($built->xml, Kembec::SunatSdk::SigningKey->new('cert.pem', 'key.pem'));
    print $built->file_name, ' ', $signed->digest_value, "\n";

    my $sol  = Kembec::SunatSdk::SolCredentials->new($ruc_y_usuario_sol, $clave_sol);
    my $sent = $sdk->send($built->file_name, $signed->xml, credentials => $sol, endpoint => $bill_service);
    if ($sent->outcome eq 'cdr') { print $sent->cdr->response_class, "\n" }
    else                         { print $sent->fault->code, "\n" }

=head1 DESCRIPTION

Arma el XML UBL 2.1 de una factura (01), boleta (03), nota de crédito (07) o de débito (08), resumen diario (RC),
comunicación de baja (RA) o guía de remisión remitente (09); lo firma con tu certificado, lo envía a la SUNAT y lee la
CDR. Es la versión para Perl del SDK L<https://github.com/Kembec/facturacion-electronica-sunat-sdk>: el núcleo en Rust
corre dentro del proceso por su ABI C con L<FFI::Platypus>, y los mismos datos dan el mismo comprobante firmado que en
.NET, Java, JavaScript, PHP, Python, Ruby, Rust y la línea de comandos.

La guía completa (instalación, envío, consultas, guía de remisión y errores) está en el README:
L<https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/bindings/perl/README.md>.

=head1 METHODS

Cada método corre un comando del contrato
(L<docs/cli.md|https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/docs/cli.md>) y devuelve un
objeto de solo lectura con un accesor por campo y C<raw_json>, la respuesta tal como la dio el SDK. Los resultados
posibles de cada comando se distinguen por C<outcome>, con los nombres del contrato (C<cdr>, C<fault>, C<no_verdict>...).

=over

=item new(http_timeout => $segundos)

=item build($document)

=item sign($unsigned_xml, $signing_key, signature_id => $id)

=item zip($name, $xml)

=item read_cdr($bytes)

=item send($name, $signed_xml, credentials => $sol, endpoint => $url)

=item summary($name, $signed_xml, credentials => $sol, endpoint => $url)

=item ticket($ticket, credentials => $sol, endpoint => $url)

=item status(ruc => ..., document_type => ..., series => ..., number => ..., credentials => $sol, endpoint => $url)

=item validity(ruc => ..., document_type => ..., series => ..., number => ..., issue_date => 'AAAA-MM-DD', total => '118.00', credentials => $api)

=item gre_send($name, $signed_xml, api => $api, sol => $sol, token_endpoint => $url, endpoint => $url)

=item gre_ticket($ticket, api => $api, sol => $sol, token_endpoint => $url, endpoint => $url)

=item native_version

=back

=head1 ERRORS

C<Kembec::SunatSdk::ArgumentError> cuando un argumento falta, es de otro tipo, está vacío, no es UTF-8 o el total es
un número de coma flotante; C<Kembec::SunatSdk::Error> cuando el comando no corrió (C<message> y C<command>); y
C<Kembec::SunatSdk::NativeLibraryError>, su subclase, cuando la biblioteca nativa no se pudo cargar.

=head1 LICENSE

MIT, © 2026 Manuel Benancio.

=cut
