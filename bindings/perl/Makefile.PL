# The distribution Kembec-SunatSdk. It carries the native library of bindings/ffi for every platform it serves
# (scripts/build-dist.sh puts them in) and installs only the one this perl loads; on a platform without one it stops
# as "OS unsupported", unless KEMBEC_SUNAT_SDK_LIBRARY names a build of bindings/ffi.
use 5.026;
use strict;
use warnings;

use ExtUtils::MakeMaker;
use File::Find ();
use lib 'lib';
use Kembec::SunatSdk::Platform;

my $key = Kembec::SunatSdk::Platform::key();
my $file = Kembec::SunatSdk::Platform::library_file() || '';
my $library = "lib/Kembec/SunatSdk/native/$key/$file";
my $override = $ENV{KEMBEC_SUNAT_SDK_LIBRARY} || '';
unless (($file && -f $library) || ($override && -f $override)) {
    print "OS unsupported: Kembec-SunatSdk carries native libraries for @Kembec::SunatSdk::Platform::SUPPORTED, not for "
        . "$key; set KEMBEC_SUNAT_SDK_LIBRARY to a build of bindings/ffi\n";
    exit 0;
}

# Every module, and of the native libraries only this platform's.
my %pm;
File::Find::find({ no_chdir => 1, wanted => sub {
    return unless -f $_;
    return if m{\Alib/Kembec/SunatSdk/native/} && $_ ne $library;
    (my $target = $_) =~ s{\Alib/}{};
    $pm{$_} = "\$(INST_LIB)/$target";
} }, 'lib');

my $web = 'https://github.com/Kembec/facturacion-electronica-sunat-sdk';
# MakeMaker calls every version with an underscore "unstable" (alpha); a beta of the SDK is a "testing" release.
my $release_status = MM->parse_version('lib/Kembec/SunatSdk.pm') =~ /_/ ? 'testing' : 'stable';
WriteMakefile(
    NAME               => 'Kembec::SunatSdk',
    AUTHOR             => 'Manuel Benancio',
    VERSION_FROM       => 'lib/Kembec/SunatSdk.pm',
    ABSTRACT           => 'SUNAT (Peru) electronic invoicing SDK: build, sign and send vouchers from Perl',
    LICENSE            => 'mit',
    MIN_PERL_VERSION   => '5.026',
    CONFIGURE_REQUIRES => { 'ExtUtils::MakeMaker' => '6.64' },
    PREREQ_PM          => {
        'FFI::Platypus' => '2.00',
        'JSON::PP'      => '2.27',
        'MIME::Base64'  => 0,
        'POSIX'         => 0,
        'Scalar::Util'  => 0,
    },
    TEST_REQUIRES => { 'Test::More' => '0.88' },
    PM            => \%pm,
    META_MERGE    => {
        'meta-spec'    => { version => 2 },
        release_status => $release_status,
        resources      => {
            homepage   => 'https://fe.kembec.com',
            repository => { type => 'git', url => "$web.git", web => $web },
            bugtracker => { web => "$web/issues" },
        },
        keywords       => [qw(sunat peru facturacion electronic-invoicing ubl xmldsig cpe)],
    },
    dist  => { COMPRESS => 'gzip -9f', SUFFIX => 'gz' },
    clean => { FILES => 'Kembec-SunatSdk-*' },
);
