using System.Collections.Generic;
using System.Text.Json;
using Kembec.SunatSdk.Results;

namespace Kembec.SunatSdk
{
    /// <summary><c>validity</c> (Consulta Integrada): the voucher's state, a refused query, or no verdict.</summary>
    public sealed class ValidityResult : SdkResult
    {
        internal ValidityResult(JsonElement value, string rawJson)
            : base(rawJson)
        {
            Outcome = Fields.ReadOutcome(value, Outcome.Validity, Outcome.QueryError, Outcome.NoVerdict);
            Observations = new List<string>();
            if (Outcome == Outcome.Validity)
            {
                StateCode = Fields.OptionalText(value, "state_code");
                State = OutcomeNames.Validity(Fields.OptionalText(value, "state"));
                TaxpayerStateCode = Fields.OptionalText(value, "taxpayer_state_code");
                DomicileConditionCode = Fields.OptionalText(value, "domicile_condition_code");
                Observations = Fields.Texts(value, "observations");
                Message = Fields.OptionalText(value, "message");
            }
            else if (Outcome == Outcome.QueryError)
            {
                QueryError = new QueryError(value);
            }
            else
            {
                NoVerdict = new NoVerdict(value);
            }
        }

        /// <summary><see cref="Outcome.Validity"/>, <see cref="Outcome.QueryError"/> or <see cref="Outcome.NoVerdict"/>.</summary>
        public Outcome Outcome { get; }

        /// <summary>SUNAT's <c>estadoCp</c> (<c>0</c> to <c>4</c>).</summary>
        public string? StateCode { get; }

        /// <summary>No state is a reason to issue the operation with another number.</summary>
        public ValidityState State { get; }

        /// <summary>SUNAT's <c>estadoRuc</c> of the issuer.</summary>
        public string? TaxpayerStateCode { get; }

        /// <summary>SUNAT's <c>condDomiRuc</c> of the issuer.</summary>
        public string? DomicileConditionCode { get; }

        /// <summary>Where the data sent differs from SUNAT's (an accepted voucher with another issue date…).</summary>
        public IReadOnlyList<string> Observations { get; }

        /// <summary>SUNAT's message.</summary>
        public string? Message { get; }

        /// <summary>Set with <see cref="Outcome.QueryError"/>.</summary>
        public QueryError? QueryError { get; }

        /// <summary>Set with <see cref="Outcome.NoVerdict"/>: ask again with the same data.</summary>
        public NoVerdict? NoVerdict { get; }
    }

    /// <summary>The Consulta Integrada refused the query (HTTP 200, 400 or 422 with <c>success: false</c>).</summary>
    public sealed class QueryError
    {
        internal QueryError(JsonElement value)
        {
            HttpStatus = Fields.Int(value, "http_status");
            ErrorCode = Fields.OptionalText(value, "error_code");
            Message = Fields.Text(value, "message");
        }

        /// <summary>The HTTP status of the answer.</summary>
        public int HttpStatus { get; }

        /// <summary>SUNAT's <c>errorCode</c> (<c>99</c>: malformed).</summary>
        public string? ErrorCode { get; }

        /// <summary>SUNAT's message.</summary>
        public string Message { get; }
    }

    /// <summary><c>gre-send</c>: the ticket of a despatch advice, an error of the GRE API, or no token.</summary>
    public sealed class GreSendResult : SdkResult
    {
        internal GreSendResult(JsonElement value, string rawJson)
            : base(rawJson)
        {
            Outcome = Fields.ReadOutcome(value, Outcome.Ticket, Outcome.ApiError, Outcome.NoVerdict);
            if (Outcome == Outcome.Ticket)
            {
                Ticket = Fields.Text(value, "ticket");
                ReceivedAt = Fields.OptionalText(value, "received_at");
                ZipName = Fields.OptionalText(value, "zip_name");
                HashZip = Fields.OptionalText(value, "hash_zip");
            }
            else if (Outcome == Outcome.ApiError)
            {
                ApiError = new GreApiError(value);
            }
            else
            {
                NoVerdict = new NoVerdict(value);
            }
        }

        /// <summary><see cref="Outcome.Ticket"/>, <see cref="Outcome.ApiError"/> or <see cref="Outcome.NoVerdict"/> (no token: nothing was sent).</summary>
        public Outcome Outcome { get; }

        /// <summary>Store it before anything else: it is the only handle on the outcome.</summary>
        public string? Ticket { get; }

        /// <summary>SUNAT's <c>fecRecepcion</c>, as it sent it.</summary>
        public string? ReceivedAt { get; }

        /// <summary>The name of the zip sent.</summary>
        public string? ZipName { get; }

        /// <summary>SHA-256 of the zip, lowercase hex.</summary>
        public string? HashZip { get; }

        /// <summary>Set with <see cref="Outcome.ApiError"/>.</summary>
        public GreApiError? ApiError { get; }

        /// <summary>Set with <see cref="Outcome.NoVerdict"/>.</summary>
        public NoVerdict? NoVerdict { get; }
    }

    /// <summary><c>gre-ticket</c>: the state of a despatch advice's ticket.</summary>
    public sealed class GreTicketResult : SdkResult
    {
        internal GreTicketResult(JsonElement value, string rawJson)
            : base(rawJson)
        {
            Outcome = Fields.ReadOutcome(value, Outcome.TicketStatus, Outcome.ApiError, Outcome.NoVerdict);
            Ticket = Fields.Text(value, "ticket");
            if (Outcome == Outcome.TicketStatus)
            {
                StatusCode = Fields.Text(value, "status_code");
                State = OutcomeNames.Ticket(Fields.OptionalText(value, "state"));
                CdrGenerated = Fields.Bool(value, "cdr_generated");
                var error = Fields.OptionalObject(value, "error");
                Error = error.HasValue ? new GreTicketError(error.Value) : null;
                var cdr = Fields.OptionalObject(value, "cdr");
                Cdr = cdr.HasValue ? new Cdr(cdr.Value) : null;
                CdrZip = Fields.OptionalBase64(value, "cdr_zip_base64");
            }
            else if (Outcome == Outcome.ApiError)
            {
                ApiError = new GreApiError(value);
            }
            else
            {
                NoVerdict = new NoVerdict(value);
            }
        }

        /// <summary><see cref="Outcome.TicketStatus"/>, <see cref="Outcome.ApiError"/> or <see cref="Outcome.NoVerdict"/>.</summary>
        public Outcome Outcome { get; }

        /// <summary>The ticket asked about.</summary>
        public string Ticket { get; }

        /// <summary>SUNAT's <c>codRespuesta</c> (<c>0</c>, <c>98</c>, <c>99</c>).</summary>
        public string? StatusCode { get; }

        /// <summary><see cref="TicketState.InProcess"/>: ask again with the same ticket, never send again.</summary>
        public TicketState State { get; }

        /// <summary>SUNAT's <c>indCdrGenerado</c> is 1.</summary>
        public bool CdrGenerated { get; }

        /// <summary>SUNAT's <c>error</c> (<c>numError</c>, <c>desError</c>), with 99.</summary>
        public GreTicketError? Error { get; }

        /// <summary>The CDR, when generated.</summary>
        public Cdr? Cdr { get; }

        /// <summary>The CDR zip, when generated.</summary>
        public byte[]? CdrZip { get; }

        /// <summary>Set with <see cref="Outcome.ApiError"/>.</summary>
        public GreApiError? ApiError { get; }

        /// <summary>Set with <see cref="Outcome.NoVerdict"/>: ask again with the same ticket.</summary>
        public NoVerdict? NoVerdict { get; }
    }

    /// <summary>The <c>error</c> of a GRE ticket: SUNAT's code and text, and its class.</summary>
    public sealed class GreTicketError
    {
        internal GreTicketError(JsonElement value)
        {
            Code = Fields.OptionalText(value, "code");
            Description = Fields.Text(value, "description");
            Class = OutcomeNames.Class(Fields.OptionalText(value, "class"));
        }

        /// <summary>SUNAT's <c>numError</c>.</summary>
        public string? Code { get; }

        /// <summary>SUNAT's <c>desError</c>.</summary>
        public string Description { get; }

        /// <summary>How the SDK classifies the code.</summary>
        public ResponseClass Class { get; }
    }

    /// <summary>An error body of the GRE REST API (manual GRE-M, §1.3 and annex I).</summary>
    public sealed class GreApiError
    {
        internal GreApiError(JsonElement value)
        {
            Stage = OutcomeNames.Stage(Fields.OptionalText(value, "stage"));
            HttpStatus = Fields.Int(value, "http_status");
            GeneralError = Fields.OptionalText(value, "general_error");
            Code = Fields.OptionalText(value, "code");
            Message = Fields.Text(value, "message");
            var errors = new List<GreApiErrorItem>();
            foreach (var item in EnumerateErrors(value))
            {
                errors.Add(new GreApiErrorItem(Fields.OptionalText(item, "code"), Fields.Text(item, "description")));
            }
            Errors = errors;
        }

        /// <summary>The call SUNAT answered: <see cref="ExchangeStage.Token"/>, <see cref="ExchangeStage.Send"/> or <see cref="ExchangeStage.Ticket"/>.</summary>
        public ExchangeStage Stage { get; }

        /// <summary>The HTTP status.</summary>
        public int HttpStatus { get; }

        /// <summary>Annex I's name of the status (<c>unauthorized</c>, <c>unprocessable_entity</c>…).</summary>
        public string? GeneralError { get; }

        /// <summary>SUNAT's <c>cod</c>.</summary>
        public string? Code { get; }

        /// <summary>SUNAT's <c>msg</c>.</summary>
        public string Message { get; }

        /// <summary>With 422, what SUNAT rejected, in its words.</summary>
        public IReadOnlyList<GreApiErrorItem> Errors { get; }

        private static IEnumerable<JsonElement> EnumerateErrors(JsonElement value)
        {
            if (value.TryGetProperty("errors", out var errors) && errors.ValueKind == JsonValueKind.Array)
            {
                foreach (var item in errors.EnumerateArray())
                {
                    yield return item;
                }
            }
        }
    }

    /// <summary>One validation error of a 422 from the GRE API.</summary>
    public sealed class GreApiErrorItem
    {
        internal GreApiErrorItem(string? code, string description)
        {
            Code = code;
            Description = description;
        }

        /// <summary>SUNAT's code.</summary>
        public string? Code { get; }

        /// <summary>SUNAT's text.</summary>
        public string Description { get; }
    }
}
