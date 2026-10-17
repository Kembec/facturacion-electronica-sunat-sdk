using System.Collections.Generic;
using System.Text.Json;

namespace Kembec.SunatSdk
{
    /// <summary>What every result keeps: the response exactly as the library gave it.</summary>
    public abstract class SdkResult
    {
        internal SdkResult(string rawJson)
        {
            RawJson = rawJson;
        }

        /// <summary>The JSON of docs/cli.md for this result, for logs and for fields newer than this package.</summary>
        public string RawJson { get; }
    }

    /// <summary>A CDR (constancia de recepción): the code SUNAT answered and its class.</summary>
    public sealed class Cdr : SdkResult
    {
        internal Cdr(JsonElement value)
            : base(value.GetRawText())
        {
            ReferenceId = Results.Fields.Text(value, "reference_id");
            ResponseCode = Results.Fields.Text(value, "response_code");
            Description = Results.Fields.Text(value, "description");
            Notes = Results.Fields.Texts(value, "notes");
            Class = Results.OutcomeNames.Class(Results.Fields.OptionalText(value, "class"));
        }

        /// <summary>The voucher it answers (<c>F001-1</c>).</summary>
        public string ReferenceId { get; }

        /// <summary>SUNAT's response code (<c>0</c>, <c>2047</c>…).</summary>
        public string ResponseCode { get; }

        /// <summary>SUNAT's description of the code.</summary>
        public string Description { get; }

        /// <summary>The notes, each <c>"code - text"</c>; with code 0, every note is an observation.</summary>
        public IReadOnlyList<string> Notes { get; }

        /// <summary>How the SDK classifies the code and the notes.</summary>
        public ResponseClass Class { get; }

        /// <summary>Code 0 without observations.</summary>
        public bool IsAccepted => Class == ResponseClass.Accepted;

        /// <summary>Accepted, with or without observations: the voucher is valid.</summary>
        public bool IsValidDocument => Class == ResponseClass.Accepted || Class == ResponseClass.Observed;
    }

    /// <summary>A SOAP fault: SUNAT refused the call with a code.</summary>
    public sealed class Fault
    {
        internal Fault(JsonElement value)
        {
            Code = Results.Fields.Text(value, "code");
            Message = Results.Fields.Text(value, "message");
            Class = Results.OutcomeNames.Class(Results.Fields.OptionalText(value, "class"));
        }

        /// <summary>SUNAT's code (<c>1033</c>, <c>0111</c>…).</summary>
        public string Code { get; }

        /// <summary>SUNAT's text.</summary>
        public string Message { get; }

        /// <summary>How the SDK classifies the code; <see cref="ResponseClass.AlreadyRegistered"/> is 1033.</summary>
        public ResponseClass Class { get; }
    }

    /// <summary>
    /// No answer, or an HTTP status that says nothing about the document. It is not a rejection: ask again
    /// (docs/cli.md says when to resend the same signed XML with the same number).
    /// </summary>
    public sealed class NoVerdict
    {
        internal NoVerdict(JsonElement value)
        {
            Reason = Results.OutcomeNames.Reason(Results.Fields.Text(value, "reason"));
            HttpStatus = Results.Fields.OptionalInt(value, "http_status");
            Message = Results.Fields.Text(value, "message");
            Stage = Results.OutcomeNames.Stage(Results.Fields.OptionalText(value, "stage"));
        }

        /// <summary>The wait ran out, or SUNAT answered an HTTP status.</summary>
        public NoVerdictReason Reason { get; }

        /// <summary>The HTTP status (401…), null after a timeout.</summary>
        public int? HttpStatus { get; }

        /// <summary>What happened, in English.</summary>
        public string Message { get; }

        /// <summary>For the REST exchanges, the call without an answer; <see cref="ExchangeStage.Token"/> means nothing was sent.</summary>
        public ExchangeStage Stage { get; }
    }
}
