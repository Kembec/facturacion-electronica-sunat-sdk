using System;
using System.Collections.Generic;
using System.Globalization;
using System.Text.Json;
using Kembec.SunatSdk.Interop;

namespace Kembec.SunatSdk
{
    /// <summary>
    /// The commands of the SDK's contract (docs/cli.md) in process, through its C ABI (docs/c-abi.md): the same
    /// validation, XML, signature, network calls and outcomes as the CLI and every other language. Immutable and
    /// safe to share between threads. Every method blocks until its command ends; the network ones wait on
    /// SUNAT up to <see cref="HttpTimeout"/>, so a UI thread should not call them.
    /// </summary>
    public sealed class SunatSdkClient
    {
        /// <summary>90 seconds, the CLI's default.</summary>
        public static readonly TimeSpan DefaultHttpTimeout = TimeSpan.FromSeconds(90);

        private const int MaxHttpTimeoutSeconds = 300;
        private readonly int _timeoutSeconds;

        /// <summary>A client that waits on SUNAT up to <see cref="DefaultHttpTimeout"/>.</summary>
        public SunatSdkClient()
            : this(DefaultHttpTimeout)
        {
        }

        /// <summary>A client that waits on SUNAT up to <paramref name="httpTimeout"/>.</summary>
        /// <param name="httpTimeout">whole seconds, 1 to 300; the token and the call of the REST commands share it</param>
        public SunatSdkClient(TimeSpan httpTimeout)
        {
            var seconds = httpTimeout.TotalSeconds;
            if (seconds < 1 || seconds > MaxHttpTimeoutSeconds || seconds != Math.Floor(seconds))
            {
                throw new ArgumentOutOfRangeException(nameof(httpTimeout), httpTimeout, "must be whole seconds from 1 to 300");
            }
            _timeoutSeconds = (int)seconds;
            HttpTimeout = httpTimeout;
        }

        /// <summary>How long a network command waits on SUNAT.</summary>
        public TimeSpan HttpTimeout { get; }

        /// <summary>
        /// The version of the native library loaded (the SDK's, <c>1.0.0-rc.8</c>); equal to this package's.
        /// Loads the library: throws <see cref="SunatSdkLoadException"/> if it cannot.
        /// </summary>
        public static string NativeVersion => Native.Version;

        /// <summary>Renders the unsigned UBL XML of one voucher, daily summary, void communication or despatch advice.</summary>
        /// <param name="documentJson">the document as JSON, in the shape of docs/input-model.md</param>
        /// <exception cref="ArgumentException">the text is not one JSON value</exception>
        /// <exception cref="SunatSdkException">the SDK rejected the document; the message says which field and why</exception>
        public BuildResult Build(string documentJson)
        {
            Require(documentJson, nameof(documentJson));
            return Native.Run("build", w => WriteRaw(w, documentJson, nameof(documentJson)), NoFlags, (v, raw) => new BuildResult(v, raw));
        }

        /// <summary>Signs XML from <see cref="Build"/> with XML-DSig (enveloped, RSA-SHA1, as SUNAT requires).</summary>
        /// <param name="unsignedXml">the XML of <see cref="BuildResult.Xml"/></param>
        /// <param name="key">the certificate and its key</param>
        /// <param name="signatureId">the signature's <c>Id</c>; <c>signatureKG</c> when null</param>
        public SignResult Sign(string unsignedXml, SigningKey key, string? signatureId = null)
        {
            Require(unsignedXml, nameof(unsignedXml));
            Require(key, nameof(key));
            var flags = new[] { new Flag("cert", key.CertificatePath), new Flag("key", key.PrivateKeyPath) };
            return Native.Run("sign", w => Object(w, "xml", unsignedXml, "signature_id", signatureId), flags, (v, raw) => new SignResult(v, raw));
        }

        /// <summary>Zips the XML under <c>&lt;name&gt;.xml</c>, as SUNAT expects it.</summary>
        /// <param name="name">the file name without extension (<c>20100066603-01-F001-1</c>)</param>
        /// <param name="xml">the signed XML</param>
        public ZipResult Zip(string name, string xml)
        {
            Require(name, nameof(name));
            Require(xml, nameof(xml));
            return Native.Run("zip", w => Object(w, "name", name, "xml", xml), NoFlags, (v, raw) => new ZipResult(v, raw));
        }

        /// <summary>Reads a CDR, zip or XML, and classifies its response code (the <c>cdr</c> command).</summary>
        /// <param name="cdr">the bytes of the CDR zip or XML</param>
        public Cdr ReadCdr(byte[] cdr)
        {
            Require(cdr, nameof(cdr));
            var encoded = Convert.ToBase64String(cdr);
            return Native.Run("cdr", w => Object(w, "cdr_base64", encoded), NoFlags, (v, raw) => new Cdr(v));
        }

        /// <summary>
        /// Zips the signed XML and sends it with sendBill. A fault is a result; a send without an answer throws
        /// <see cref="SunatSdkException"/>, since it may have arrived: send the same XML again, never another number.
        /// </summary>
        /// <param name="name">the file name of <see cref="BuildResult.FileName"/></param>
        /// <param name="signedXml">the XML of <see cref="SignResult.Xml"/></param>
        /// <param name="credentials">the SOL user and password</param>
        /// <param name="endpoint">the billService URL, https only</param>
        public SendResult Send(string name, string signedXml, SolCredentials credentials, string endpoint)
        {
            Require(name, nameof(name));
            Require(signedXml, nameof(signedXml));
            var flags = Soap(credentials, endpoint);
            return Native.Run("send", w => Object(w, "name", name, "xml", signedXml), flags, (v, raw) => new SendResult(v, raw));
        }

        /// <summary>
        /// Zips a signed daily summary (RC) or void communication (RA) and sends it with sendSummary. Store the
        /// ticket before anything else. A send without an answer throws, as <see cref="Send"/>.
        /// </summary>
        /// <param name="name"><c>RUC-RC|RA-YYYYMMDD-N</c></param>
        /// <param name="signedXml">the signed summary or void</param>
        /// <param name="credentials">the SOL user and password</param>
        /// <param name="endpoint">the billService URL, https only</param>
        public SummaryResult Summary(string name, string signedXml, SolCredentials credentials, string endpoint)
        {
            Require(name, nameof(name));
            Require(signedXml, nameof(signedXml));
            var flags = Soap(credentials, endpoint);
            return Native.Run("summary", w => Object(w, "name", name, "xml", signedXml), flags, (v, raw) => new SummaryResult(v, raw));
        }

        /// <summary>Asks getStatus for a summary's or a void's ticket. No answer is <see cref="Outcome.NoVerdict"/>: ask again, never send again.</summary>
        /// <param name="ticket">the ticket of <see cref="SummaryResult.Ticket"/></param>
        /// <param name="credentials">the SOL user and password</param>
        /// <param name="endpoint">the billService URL, https only</param>
        public TicketResult Ticket(string ticket, SolCredentials credentials, string endpoint)
        {
            Require(ticket, nameof(ticket));
            var flags = Soap(credentials, endpoint);
            return Native.Run("ticket", w => Object(w, "ticket", ticket), flags, (v, raw) => new TicketResult(v, raw));
        }

        /// <summary>Asks getStatusCdr for one voucher, with its CDR when SUNAT attaches it. No answer is <see cref="Outcome.NoVerdict"/>.</summary>
        /// <param name="ruc">the issuer's RUC</param>
        /// <param name="documentType"><c>01</c>, <c>03</c>, <c>07</c> or <c>08</c></param>
        /// <param name="series"><c>F001</c></param>
        /// <param name="number">the correlative number</param>
        /// <param name="credentials">the SOL user and password</param>
        /// <param name="endpoint">the billConsultService URL, https only</param>
        public StatusResult Status(string ruc, string documentType, string series, int number, SolCredentials credentials, string endpoint)
        {
            Require(ruc, nameof(ruc));
            Require(documentType, nameof(documentType));
            Require(series, nameof(series));
            var flags = Soap(credentials, endpoint);
            return Native.Run(
                "status",
                w => VoucherKey(w, ruc, documentType, series, number, end: true),
                flags,
                (v, raw) => new StatusResult(v, raw));
        }

        /// <summary>
        /// Asks SUNAT's Consulta Integrada whether a voucher (01, 03, 07, 08) exists and in which state: a token,
        /// then the query, both inside <see cref="HttpTimeout"/>. There is no test environment: this is production.
        /// </summary>
        /// <param name="ruc">the issuer's RUC</param>
        /// <param name="documentType"><c>01</c>, <c>03</c>, <c>07</c> or <c>08</c></param>
        /// <param name="series"><c>F001</c></param>
        /// <param name="number">the correlative number</param>
        /// <param name="issueDate">the issue date; only the date is sent</param>
        /// <param name="total">the voucher's total (PayableAmount), up to 2 decimals</param>
        /// <param name="credentials">the Consulta Integrada pair of «Credenciales de API SUNAT»</param>
        /// <param name="requesterRuc">the RUC that asks; the issuer's when null</param>
        /// <param name="tokenEndpoint">replaces the production token base (https only); null keeps it</param>
        /// <param name="endpoint">replaces the production query base (https only); null keeps it</param>
        public ValidityResult Validity(
            string ruc,
            string documentType,
            string series,
            int number,
            DateTime issueDate,
            decimal total,
            ApiCredentials credentials,
            string? requesterRuc = null,
            string? tokenEndpoint = null,
            string? endpoint = null)
        {
            Require(ruc, nameof(ruc));
            Require(documentType, nameof(documentType));
            Require(series, nameof(series));
            Require(credentials, nameof(credentials));
            var flags = new List<Flag>
            {
                new Flag("client-id", credentials.ClientId),
                new Flag("client-secret", credentials.ClientSecret),
                new Flag("timeout", _timeoutSeconds),
            };
            if (tokenEndpoint != null)
            {
                flags.Add(new Flag("token-endpoint", tokenEndpoint));
            }
            if (endpoint != null)
            {
                flags.Add(new Flag("endpoint", endpoint));
            }
            var date = issueDate.ToString("yyyy-MM-dd", CultureInfo.InvariantCulture);
            var amount = total.ToString(CultureInfo.InvariantCulture);
            return Native.Run(
                "validity",
                w =>
                {
                    VoucherKey(w, ruc, documentType, series, number, end: false);
                    w.WriteString("issue_date", date);
                    w.WriteString("total", amount);
                    if (requesterRuc != null)
                    {
                        w.WriteString("requester_ruc", requesterRuc);
                    }
                    w.WriteEndObject();
                },
                flags,
                (v, raw) => new ValidityResult(v, raw));
        }

        /// <summary>
        /// Sends a signed despatch advice (09) by SUNAT's GRE REST API: a token by password, then the zip with its
        /// hash. SUNAT has no test environment for GRE and the endpoints have no default. A send without an answer
        /// throws: send the same file again, never the operation with another number.
        /// </summary>
        /// <param name="name"><c>RUC-09-SERIE-NUMERO</c></param>
        /// <param name="signedXml">the signed guide</param>
        /// <param name="api">the GRE pair of «Credenciales de API SUNAT»</param>
        /// <param name="sol">the RUC followed by the SOL user, and its password</param>
        /// <param name="tokenEndpoint">the token base, https only (<c>https://api-seguridad.sunat.gob.pe/v1/clientessol</c>)</param>
        /// <param name="endpoint">the API base, https only (<c>https://api-cpe.sunat.gob.pe/v1</c>)</param>
        public GreSendResult GreSend(string name, string signedXml, ApiCredentials api, SolCredentials sol, string tokenEndpoint, string endpoint)
        {
            Require(name, nameof(name));
            Require(signedXml, nameof(signedXml));
            var flags = Gre(api, sol, tokenEndpoint, endpoint);
            return Native.Run("gre-send", w => Object(w, "name", name, "xml", signedXml), flags, (v, raw) => new GreSendResult(v, raw));
        }

        /// <summary>Asks for the outcome of a sent guide by its ticket (a token, then the query). 98 is in process: ask again with the same ticket.</summary>
        /// <param name="ticket">the ticket of <see cref="GreSendResult.Ticket"/></param>
        /// <param name="api">the GRE pair of «Credenciales de API SUNAT»</param>
        /// <param name="sol">the RUC followed by the SOL user, and its password</param>
        /// <param name="tokenEndpoint">the token base, https only</param>
        /// <param name="endpoint">the API base, https only</param>
        public GreTicketResult GreTicket(string ticket, ApiCredentials api, SolCredentials sol, string tokenEndpoint, string endpoint)
        {
            Require(ticket, nameof(ticket));
            var flags = Gre(api, sol, tokenEndpoint, endpoint);
            return Native.Run("gre-ticket", w => Object(w, "ticket", ticket), flags, (v, raw) => new GreTicketResult(v, raw));
        }

        private static readonly Flag[] NoFlags = new Flag[0];

        private Flag[] Soap(SolCredentials credentials, string endpoint)
        {
            Require(credentials, nameof(credentials));
            Require(endpoint, nameof(endpoint));
            return new[]
            {
                new Flag("endpoint", endpoint),
                new Flag("user", credentials.User),
                new Flag("password", credentials.Password),
                new Flag("timeout", _timeoutSeconds),
            };
        }

        private Flag[] Gre(ApiCredentials api, SolCredentials sol, string tokenEndpoint, string endpoint)
        {
            Require(api, nameof(api));
            Require(sol, nameof(sol));
            Require(tokenEndpoint, nameof(tokenEndpoint));
            Require(endpoint, nameof(endpoint));
            return new[]
            {
                new Flag("client-id", api.ClientId),
                new Flag("client-secret", api.ClientSecret),
                new Flag("user", sol.User),
                new Flag("password", sol.Password),
                new Flag("token-endpoint", tokenEndpoint),
                new Flag("endpoint", endpoint),
                new Flag("timeout", _timeoutSeconds),
            };
        }

        private static void VoucherKey(Utf8JsonWriter w, string ruc, string documentType, string series, int number, bool end)
        {
            w.WriteStartObject();
            w.WriteString("ruc", ruc);
            w.WriteString("document_type", documentType);
            w.WriteString("series", series);
            w.WriteNumber("number", number);
            if (end)
            {
                w.WriteEndObject();
            }
        }

        /// <summary>An object of string fields given as name, value, name, value…; a null value is left out.</summary>
        private static void Object(Utf8JsonWriter w, params string?[] namesAndValues)
        {
            w.WriteStartObject();
            for (var i = 0; i + 1 < namesAndValues.Length; i += 2)
            {
                if (namesAndValues[i + 1] != null)
                {
                    w.WriteString(namesAndValues[i]!, namesAndValues[i + 1]);
                }
            }
            w.WriteEndObject();
        }

        private static void WriteRaw(Utf8JsonWriter w, string json, string parameter)
        {
            try
            {
                w.WriteRawValue(json);
            }
            catch (JsonException e)
            {
                throw new ArgumentException("is not one JSON value: " + e.Message, parameter, e);
            }
        }

        private static void Require(object value, string name)
        {
            if (value == null)
            {
                throw new ArgumentNullException(name);
            }
        }
    }
}
