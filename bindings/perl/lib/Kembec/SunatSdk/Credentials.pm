package Kembec::SunatSdk::SigningKey;

# Paths of the PEM certificate and its unencrypted RSA private key. The library reads the files; their content never
# passes through Perl.

use strict;
use warnings;

use Kembec::SunatSdk::Arguments;

use overload '""' => sub { ref($_[0]) . '=[redacted]' }, bool => sub { 1 }, fallback => 1;

sub new {
    my ($class, $certificate_path, $private_key_path) = @_;
    return bless {
        certificate_path => Kembec::SunatSdk::Arguments::nonempty($certificate_path, 'certificate_path'),
        private_key_path => Kembec::SunatSdk::Arguments::nonempty($private_key_path, 'private_key_path'),
    }, $class;
}

sub certificate_path { $_[0]->{certificate_path} }
sub private_key_path { $_[0]->{private_key_path} }

package Kembec::SunatSdk::SolCredentials;

# The SOL user and password for send, summary, ticket, status and the GRE token. They reach the library inside the
# request, whose bytes are zeroed after the call, and never a result, an error or the object's text.

use strict;
use warnings;

use Kembec::SunatSdk::Arguments;

use overload '""' => sub { ref($_[0]) . '=[redacted]' }, bool => sub { 1 }, fallback => 1;

# $user: the RUC followed by the SOL user ("20100066603MODDATOS"); $password: the SOL password.
sub new {
    my ($class, $user, $password) = @_;
    my %secret = (
        user     => Kembec::SunatSdk::Arguments::nonempty($user, 'user'),
        password => Kembec::SunatSdk::Arguments::nonempty($password, 'password'),
    );
    return bless sub { \%secret }, $class;
}

# The flags of the C ABI; for the client only.
sub _flags {
    my $secret = $_[0]->();
    return { user => $secret->{user}, password => $secret->{password} };
}

package Kembec::SunatSdk::ApiCredentials;

# Client id and secret of «Credenciales de API SUNAT» (SOL): the Consulta Integrada pair for validity, or the GRE pair
# for gre_send and gre_ticket. Not the SOL user.

use strict;
use warnings;

use Kembec::SunatSdk::Arguments;

use overload '""' => sub { ref($_[0]) . '=[redacted]' }, bool => sub { 1 }, fallback => 1;

sub new {
    my ($class, $client_id, $client_secret) = @_;
    my %secret = (
        client_id     => Kembec::SunatSdk::Arguments::nonempty($client_id, 'client_id'),
        client_secret => Kembec::SunatSdk::Arguments::nonempty($client_secret, 'client_secret'),
    );
    return bless sub { \%secret }, $class;
}

sub _flags {
    my $secret = $_[0]->();
    return { 'client-id' => $secret->{client_id}, 'client-secret' => $secret->{client_secret} };
}

1;
