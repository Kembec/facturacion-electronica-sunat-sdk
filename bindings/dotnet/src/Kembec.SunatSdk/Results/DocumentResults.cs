using System.Text.Json;

namespace Kembec.SunatSdk
{
    /// <summary><c>build</c>: the unsigned UBL XML and the SUNAT file name without extension.</summary>
    public sealed class BuildResult : SdkResult
    {
        internal BuildResult(JsonElement value, string rawJson)
            : base(rawJson)
        {
            FileName = Results.Fields.Text(value, "file_name");
            Xml = Results.Fields.Text(value, "xml");
        }

        /// <summary><c>20123456789-01-F001-1</c>, or <c>RUC-RC|RA-YYYYMMDD-N</c> for summaries and voids.</summary>
        public string FileName { get; }

        /// <summary>The unsigned XML, UTF-8.</summary>
        public string Xml { get; }
    }

    /// <summary><c>sign</c>: the signed XML and the two values of its signature.</summary>
    public sealed class SignResult : SdkResult
    {
        internal SignResult(JsonElement value, string rawJson)
            : base(rawJson)
        {
            Xml = Results.Fields.Text(value, "xml");
            DigestValue = Results.Fields.Text(value, "digest_value");
            SignatureValue = Results.Fields.Text(value, "signature_value");
        }

        /// <summary>The signed XML.</summary>
        public string Xml { get; }

        /// <summary>The <c>DigestValue</c>, base64: the hash printed on the voucher's representation.</summary>
        public string DigestValue { get; }

        /// <summary>The <c>SignatureValue</c>, base64.</summary>
        public string SignatureValue { get; }
    }

    /// <summary><c>zip</c>: the zip SUNAT expects, with the XML inside under <c>&lt;name&gt;.xml</c>.</summary>
    public sealed class ZipResult : SdkResult
    {
        internal ZipResult(JsonElement value, string rawJson)
            : base(rawJson)
        {
            FileName = Results.Fields.Text(value, "file_name");
            Content = Results.Fields.OptionalBase64(value, "zip_base64") ?? throw new Results.ResponseShapeException("`zip_base64` is missing");
        }

        /// <summary><c>20123456789-01-F001-1.zip</c>.</summary>
        public string FileName { get; }

        /// <summary>The zip's bytes.</summary>
        public byte[] Content { get; }
    }
}
