namespace Kembec.SunatSdk
{
    /// <summary>The <c>outcome</c> of a network command (docs/cli.md). Each result says which it can carry.</summary>
    public enum Outcome
    {
        /// <summary><c>send</c>: SUNAT answered with a CDR.</summary>
        Cdr,

        /// <summary>A SOAP fault: SUNAT refused the call with a code (<c>1033</c>, <c>0111</c>…).</summary>
        Fault,

        /// <summary><c>summary</c> and <c>gre-send</c>: SUNAT received the file and gave a ticket.</summary>
        Ticket,

        /// <summary><c>ticket</c> and <c>gre-ticket</c>: the state of a ticket.</summary>
        TicketStatus,

        /// <summary><c>status</c>: getStatusCdr's answer.</summary>
        Status,

        /// <summary>No answer, or an HTTP status that says nothing about the document: ask again.</summary>
        NoVerdict,

        /// <summary><c>validity</c>: the voucher's state in the Consulta Integrada.</summary>
        Validity,

        /// <summary><c>validity</c>: SUNAT refused the query (not the voucher).</summary>
        QueryError,

        /// <summary><c>gre-send</c> and <c>gre-ticket</c>: an error body of the GRE REST API.</summary>
        ApiError,
    }

    /// <summary>The class of a SUNAT response code, as the SDK classifies a CDR or a fault.</summary>
    public enum ResponseClass
    {
        /// <summary>A code the SDK does not classify, or none.</summary>
        Unknown,

        /// <summary>Code 0 without observations.</summary>
        Accepted,

        /// <summary>4000 and above, or code 0 with any note: accepted, with observations.</summary>
        Observed,

        /// <summary>100 to 1999 but 1032 and 1033: the file was not processed and the document counts as not received.</summary>
        Exception,

        /// <summary>1033: SUNAT already has it; fetch its CDR with <c>Status</c>.</summary>
        AlreadyRegistered,

        /// <summary>1032: the number was voided or rejected before.</summary>
        NumberVoidedOrRejected,

        /// <summary>2000 to 3999 but 2323 and 2324: rejected; for invoices and notes the number is spent.</summary>
        Rejected,

        /// <summary>2323, 2324: the void was already reported.</summary>
        VoidAlreadyReported,
    }

    /// <summary>The state of a ticket.</summary>
    public enum TicketState
    {
        /// <summary>A status code the contract does not name, or no state.</summary>
        Unknown,

        /// <summary>0: processed.</summary>
        Processed,

        /// <summary>98: in process; ask again with the same ticket, never send again.</summary>
        InProcess,

        /// <summary>99: processed with errors.</summary>
        ProcessedWithErrors,
    }

    /// <summary>The voucher's state in SUNAT's Consulta Integrada.</summary>
    public enum ValidityState
    {
        /// <summary>A state code the contract does not name, or none.</summary>
        Unknown,

        /// <summary>0: not reported. It does not prove the voucher never arrived.</summary>
        NotReported,

        /// <summary>1: accepted.</summary>
        Accepted,

        /// <summary>2: voided.</summary>
        Voided,

        /// <summary>3: printed, authorized.</summary>
        PrintedAuthorized,

        /// <summary>4: printed, not authorized.</summary>
        PrintedNotAuthorized,
    }

    /// <summary>Why there is no verdict.</summary>
    public enum NoVerdictReason
    {
        /// <summary>The wait ran out.</summary>
        Timeout,

        /// <summary>SUNAT answered an HTTP status that says nothing about the document (401 included).</summary>
        HttpStatus,
    }

    /// <summary>The call of a REST exchange (Consulta Integrada or GRE) an outcome belongs to.</summary>
    public enum ExchangeStage
    {
        /// <summary>A SOAP exchange, which has a single call.</summary>
        None,

        /// <summary>The OAuth token: nothing was sent.</summary>
        Token,

        /// <summary>The Consulta Integrada query.</summary>
        Query,

        /// <summary>The GRE send.</summary>
        Send,

        /// <summary>The GRE ticket query.</summary>
        Ticket,
    }
}

namespace Kembec.SunatSdk.Results
{
    using System;

    /// <summary>The contract's names of each enum value.</summary>
    internal static class OutcomeNames
    {
        internal static string Of(Outcome outcome)
        {
            switch (outcome)
            {
                case Outcome.Cdr: return "cdr";
                case Outcome.Fault: return "fault";
                case Outcome.Ticket: return "ticket";
                case Outcome.TicketStatus: return "ticket_status";
                case Outcome.Status: return "status";
                case Outcome.NoVerdict: return "no_verdict";
                case Outcome.Validity: return "validity";
                case Outcome.QueryError: return "query_error";
                case Outcome.ApiError: return "api_error";
                default: throw new ArgumentOutOfRangeException(nameof(outcome));
            }
        }

        internal static ResponseClass Class(string? name)
        {
            switch (name)
            {
                case "accepted": return ResponseClass.Accepted;
                case "observed": return ResponseClass.Observed;
                case "exception": return ResponseClass.Exception;
                case "already_registered": return ResponseClass.AlreadyRegistered;
                case "number_voided_or_rejected": return ResponseClass.NumberVoidedOrRejected;
                case "rejected": return ResponseClass.Rejected;
                case "void_already_reported": return ResponseClass.VoidAlreadyReported;
                default: return ResponseClass.Unknown;
            }
        }

        internal static TicketState Ticket(string? name)
        {
            switch (name)
            {
                case "processed": return TicketState.Processed;
                case "in_process": return TicketState.InProcess;
                case "processed_with_errors": return TicketState.ProcessedWithErrors;
                default: return TicketState.Unknown;
            }
        }

        internal static ValidityState Validity(string? name)
        {
            switch (name)
            {
                case "not_reported": return ValidityState.NotReported;
                case "accepted": return ValidityState.Accepted;
                case "voided": return ValidityState.Voided;
                case "printed_authorized": return ValidityState.PrintedAuthorized;
                case "printed_not_authorized": return ValidityState.PrintedNotAuthorized;
                default: return ValidityState.Unknown;
            }
        }

        internal static ExchangeStage Stage(string? name)
        {
            switch (name)
            {
                case null: return ExchangeStage.None;
                case "token": return ExchangeStage.Token;
                case "query": return ExchangeStage.Query;
                case "send": return ExchangeStage.Send;
                case "ticket": return ExchangeStage.Ticket;
                default: throw new ResponseShapeException("unknown stage `" + name + "`");
            }
        }

        internal static NoVerdictReason Reason(string name)
        {
            switch (name)
            {
                case "timeout": return NoVerdictReason.Timeout;
                case "http_status": return NoVerdictReason.HttpStatus;
                default: throw new ResponseShapeException("unknown no_verdict reason `" + name + "`");
            }
        }
    }
}
