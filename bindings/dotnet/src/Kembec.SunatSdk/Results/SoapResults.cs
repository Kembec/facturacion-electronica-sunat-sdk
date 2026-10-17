using System.Text.Json;
using Kembec.SunatSdk.Results;

namespace Kembec.SunatSdk
{
    /// <summary><c>send</c> (sendBill): the CDR, or SUNAT's fault. A send without an answer throws: it may have arrived.</summary>
    public sealed class SendResult : SdkResult
    {
        internal SendResult(JsonElement value, string rawJson)
            : base(rawJson)
        {
            Outcome = Fields.ReadOutcome(value, Outcome.Cdr, Outcome.Fault);
            if (Outcome == Outcome.Cdr)
            {
                Cdr = new Cdr(Fields.OptionalObject(value, "cdr") ?? throw new ResponseShapeException("`cdr` is missing"));
                CdrZip = Fields.OptionalBase64(value, "cdr_zip_base64");
            }
            else
            {
                Fault = new Fault(value);
            }
        }

        /// <summary><see cref="Outcome.Cdr"/> or <see cref="Outcome.Fault"/>.</summary>
        public Outcome Outcome { get; }

        /// <summary>The CDR read; set with <see cref="Outcome.Cdr"/>.</summary>
        public Cdr? Cdr { get; }

        /// <summary>The CDR zip as SUNAT sent it, to store; set with <see cref="Outcome.Cdr"/>.</summary>
        public byte[]? CdrZip { get; }

        /// <summary>Set with <see cref="Outcome.Fault"/>.</summary>
        public Fault? Fault { get; }
    }

    /// <summary><c>summary</c> (sendSummary): the ticket, or SUNAT's fault.</summary>
    public sealed class SummaryResult : SdkResult
    {
        internal SummaryResult(JsonElement value, string rawJson)
            : base(rawJson)
        {
            Outcome = Fields.ReadOutcome(value, Outcome.Ticket, Outcome.Fault);
            if (Outcome == Outcome.Ticket)
            {
                Ticket = Fields.Text(value, "ticket");
            }
            else
            {
                Fault = new Fault(value);
            }
        }

        /// <summary><see cref="Outcome.Ticket"/> or <see cref="Outcome.Fault"/>.</summary>
        public Outcome Outcome { get; }

        /// <summary>Store it before anything else: it is the only handle on the outcome.</summary>
        public string? Ticket { get; }

        /// <summary>Set with <see cref="Outcome.Fault"/>.</summary>
        public Fault? Fault { get; }
    }

    /// <summary><c>ticket</c> (getStatus): the state of a summary's or a void's ticket.</summary>
    public sealed class TicketResult : SdkResult
    {
        internal TicketResult(JsonElement value, string rawJson)
            : base(rawJson)
        {
            Outcome = Fields.ReadOutcome(value, Outcome.TicketStatus, Outcome.Fault, Outcome.NoVerdict);
            Ticket = Fields.OptionalText(value, "ticket");
            if (Outcome == Outcome.TicketStatus)
            {
                StatusCode = Fields.Text(value, "status_code");
                State = OutcomeNames.Ticket(Fields.OptionalText(value, "state"));
                var cdr = Fields.OptionalObject(value, "cdr");
                Cdr = cdr.HasValue ? new Cdr(cdr.Value) : null;
                CdrZip = Fields.OptionalBase64(value, "cdr_zip_base64");
            }
            else if (Outcome == Outcome.Fault)
            {
                Fault = new Fault(value);
            }
            else
            {
                NoVerdict = new NoVerdict(value);
            }
        }

        /// <summary><see cref="Outcome.TicketStatus"/>, <see cref="Outcome.Fault"/> or <see cref="Outcome.NoVerdict"/>.</summary>
        public Outcome Outcome { get; }

        /// <summary>The ticket asked about; set with <see cref="Outcome.NoVerdict"/>.</summary>
        public string? Ticket { get; }

        /// <summary>SUNAT's status code (<c>0</c>, <c>98</c>, <c>99</c>).</summary>
        public string? StatusCode { get; }

        /// <summary><see cref="TicketState.InProcess"/>: ask again with the same ticket, never send again.</summary>
        public TicketState State { get; }

        /// <summary>The CDR, when SUNAT attaches it.</summary>
        public Cdr? Cdr { get; }

        /// <summary>The CDR zip, when SUNAT attaches it.</summary>
        public byte[]? CdrZip { get; }

        /// <summary>Set with <see cref="Outcome.Fault"/>.</summary>
        public Fault? Fault { get; }

        /// <summary>Set with <see cref="Outcome.NoVerdict"/>: ask again; the ticket proves the file arrived.</summary>
        public NoVerdict? NoVerdict { get; }
    }

    /// <summary><c>status</c> (getStatusCdr): one voucher's status, with its CDR when SUNAT attaches it.</summary>
    public sealed class StatusResult : SdkResult
    {
        internal StatusResult(JsonElement value, string rawJson)
            : base(rawJson)
        {
            Outcome = Fields.ReadOutcome(value, Outcome.Status, Outcome.Fault, Outcome.NoVerdict);
            if (Outcome == Outcome.Status)
            {
                StatusCode = Fields.Text(value, "status_code");
                StatusMessage = Fields.OptionalText(value, "status_message");
                var cdr = Fields.OptionalObject(value, "cdr");
                Cdr = cdr.HasValue ? new Cdr(cdr.Value) : null;
                CdrZip = Fields.OptionalBase64(value, "cdr_zip_base64");
            }
            else if (Outcome == Outcome.Fault)
            {
                Fault = new Fault(value);
            }
            else
            {
                NoVerdict = new NoVerdict(value);
            }
        }

        /// <summary><see cref="Outcome.Status"/>, <see cref="Outcome.Fault"/> or <see cref="Outcome.NoVerdict"/>.</summary>
        public Outcome Outcome { get; }

        /// <summary>getStatusCdr's <c>statusCode</c> (<c>0001</c>: the voucher exists and has a CDR…).</summary>
        public string? StatusCode { get; }

        /// <summary>getStatusCdr's <c>statusMessage</c>.</summary>
        public string? StatusMessage { get; }

        /// <summary>The CDR, when SUNAT attaches it.</summary>
        public Cdr? Cdr { get; }

        /// <summary>The CDR zip, when SUNAT attaches it.</summary>
        public byte[]? CdrZip { get; }

        /// <summary>Set with <see cref="Outcome.Fault"/>.</summary>
        public Fault? Fault { get; }

        /// <summary>Set with <see cref="Outcome.NoVerdict"/>: ask again.</summary>
        public NoVerdict? NoVerdict { get; }
    }
}
