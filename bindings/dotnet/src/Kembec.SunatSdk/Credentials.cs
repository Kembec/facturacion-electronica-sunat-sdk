using System;

namespace Kembec.SunatSdk
{
    /// <summary>
    /// Paths of the PEM certificate and its unencrypted RSA private key. The library reads the files; their
    /// content never passes through .NET.
    /// </summary>
    public sealed class SigningKey
    {
        /// <summary>Creates the key from two file paths.</summary>
        public SigningKey(string certificatePath, string privateKeyPath)
        {
            CertificatePath = NotEmpty(certificatePath, nameof(certificatePath));
            PrivateKeyPath = NotEmpty(privateKeyPath, nameof(privateKeyPath));
        }

        /// <summary>The certificate, PEM.</summary>
        public string CertificatePath { get; }

        /// <summary>The unencrypted RSA private key, PEM.</summary>
        public string PrivateKeyPath { get; }

        /// <summary>Hides the paths.</summary>
        public override string ToString() => "SigningKey([redacted])";

        internal static string NotEmpty(string value, string name)
        {
            if (value == null)
            {
                throw new ArgumentNullException(name);
            }
            if (value.Length == 0)
            {
                throw new ArgumentException("must not be empty", name);
            }
            return value;
        }
    }

    /// <summary>
    /// The SOL user and password for sendBill, sendSummary, getStatus, getStatusCdr and the GRE token. They go
    /// to the library inside the request, whose bytes are zeroed after the call, and never into a result, an
    /// exception or <see cref="ToString"/>.
    /// </summary>
    public sealed class SolCredentials
    {
        /// <summary>Creates the credentials.</summary>
        /// <param name="user">the RUC followed by the SOL user (<c>20100066603MODDATOS</c>)</param>
        /// <param name="password">the SOL password</param>
        public SolCredentials(string user, string password)
        {
            User = SigningKey.NotEmpty(user, nameof(user));
            Password = SigningKey.NotEmpty(password, nameof(password));
        }

        internal string User { get; }

        internal string Password { get; }

        /// <summary>Hides both values.</summary>
        public override string ToString() => "SolCredentials([redacted])";
    }

    /// <summary>
    /// Client id and secret of «Credenciales de API SUNAT» (SOL): the Consulta Integrada pair for
    /// <c>Validity</c>, or the GRE pair for <c>GreSend</c> and <c>GreTicket</c>. Not the SOL user.
    /// </summary>
    public sealed class ApiCredentials
    {
        /// <summary>Creates the credentials.</summary>
        public ApiCredentials(string clientId, string clientSecret)
        {
            ClientId = SigningKey.NotEmpty(clientId, nameof(clientId));
            ClientSecret = SigningKey.NotEmpty(clientSecret, nameof(clientSecret));
        }

        internal string ClientId { get; }

        internal string ClientSecret { get; }

        /// <summary>Hides both values.</summary>
        public override string ToString() => "ApiCredentials([redacted])";
    }
}
