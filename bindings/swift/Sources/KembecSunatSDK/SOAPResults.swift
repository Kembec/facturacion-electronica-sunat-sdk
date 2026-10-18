import Foundation

// The results of the SOAP commands (send, summary, ticket, status): one enum case per outcome of docs/cli.md, decoded
// from and encoded to the same flat JSON object, `outcome` included.

/// What `send` got: the CDR, or a SOAP fault. A send with no answer is not here: it throws, since it may have arrived.
public struct SendResult: Codable, Sendable, Hashable, RawJSONCarrying {
    public enum Outcome: Sendable, Hashable {
        /// SUNAT answered with a CDR; `zip` is the CDR as it came, to store.
        case cdr(CDR, zip: Data?)
        case fault(Fault)
    }

    public let outcome: Outcome
    /// The input field behind each code SUNAT answered; empty when the response carries none.
    public let diagnosis: [Diagnosis]
    public internal(set) var rawJSON: String = ""

    private enum CodingKeys: String, CodingKey {
        case outcome, cdr, cdrZip = "cdr_zip_base64", diagnosis
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        diagnosis = try container.decodeIfPresent([Diagnosis].self, forKey: .diagnosis) ?? []
        switch try container.decode(String.self, forKey: .outcome) {
        case "cdr":
            outcome = .cdr(try container.decode(CDR.self, forKey: .cdr), zip: try container.decodeIfPresent(Data.self, forKey: .cdrZip))
        case "fault":
            outcome = .fault(try Fault(from: decoder))
        case let other:
            throw unknownOutcome(other, container, CodingKeys.outcome)
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        if !diagnosis.isEmpty { try container.encode(diagnosis, forKey: .diagnosis) }
        switch outcome {
        case let .cdr(cdr, zip):
            try container.encode("cdr", forKey: .outcome)
            try container.encode(cdr, forKey: .cdr)
            try container.encodeIfPresent(zip, forKey: .cdrZip)
        case let .fault(fault):
            try container.encode("fault", forKey: .outcome)
            try fault.encode(to: encoder)
        }
    }
}

/// What `summary` got: the ticket (store it before anything else), or a SOAP fault.
public struct SummaryResult: Codable, Sendable, Hashable, RawJSONCarrying {
    public enum Outcome: Sendable, Hashable {
        case ticket(String)
        case fault(Fault)
    }

    public let outcome: Outcome
    /// The input field behind each code SUNAT answered; empty when the response carries none.
    public let diagnosis: [Diagnosis]
    public internal(set) var rawJSON: String = ""

    private enum CodingKeys: String, CodingKey {
        case outcome, ticket, diagnosis
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        diagnosis = try container.decodeIfPresent([Diagnosis].self, forKey: .diagnosis) ?? []
        switch try container.decode(String.self, forKey: .outcome) {
        case "ticket": outcome = .ticket(try container.decode(String.self, forKey: .ticket))
        case "fault": outcome = .fault(try Fault(from: decoder))
        case let other: throw unknownOutcome(other, container, CodingKeys.outcome)
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        if !diagnosis.isEmpty { try container.encode(diagnosis, forKey: .diagnosis) }
        switch outcome {
        case let .ticket(ticket):
            try container.encode("ticket", forKey: .outcome)
            try container.encode(ticket, forKey: .ticket)
        case let .fault(fault):
            try container.encode("fault", forKey: .outcome)
            try fault.encode(to: encoder)
        }
    }
}

/// The state of a summary's or a void's ticket (getStatus).
public struct TicketStatus: Codable, Sendable, Hashable {
    /// `0`, `98`, `99`
    public let statusCode: String
    public let state: TicketState
    public let cdr: CDR?
    public let cdrZip: Data?

    private enum CodingKeys: String, CodingKey {
        case statusCode = "status_code", state, cdr, cdrZip = "cdr_zip_base64"
    }
}

/// What `ticket` got: the ticket's state, a SOAP fault, or no verdict (ask again; never send the summary again).
public struct TicketResult: Codable, Sendable, Hashable, RawJSONCarrying {
    public enum Outcome: Sendable, Hashable {
        case ticketStatus(TicketStatus)
        case fault(Fault)
        case noVerdict(NoVerdict)
    }

    public let outcome: Outcome
    /// The input field behind each code SUNAT answered; empty when the response carries none.
    public let diagnosis: [Diagnosis]
    /// The ticket asked for, when the response repeats it (no verdict).
    public let ticket: String?
    public internal(set) var rawJSON: String = ""

    private enum CodingKeys: String, CodingKey {
        case outcome, ticket, diagnosis
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        diagnosis = try container.decodeIfPresent([Diagnosis].self, forKey: .diagnosis) ?? []
        ticket = try container.decodeIfPresent(String.self, forKey: .ticket)
        switch try container.decode(String.self, forKey: .outcome) {
        case "ticket_status": outcome = .ticketStatus(try TicketStatus(from: decoder))
        case "fault": outcome = .fault(try Fault(from: decoder))
        case "no_verdict": outcome = .noVerdict(try NoVerdict(from: decoder))
        case let other: throw unknownOutcome(other, container, CodingKeys.outcome)
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        if !diagnosis.isEmpty { try container.encode(diagnosis, forKey: .diagnosis) }
        try container.encodeIfPresent(ticket, forKey: .ticket)
        switch outcome {
        case let .ticketStatus(status):
            try container.encode("ticket_status", forKey: .outcome)
            try status.encode(to: encoder)
        case let .fault(fault):
            try container.encode("fault", forKey: .outcome)
            try fault.encode(to: encoder)
        case let .noVerdict(noVerdict):
            try container.encode("no_verdict", forKey: .outcome)
            try noVerdict.encode(to: encoder)
        }
    }
}

/// The status of one voucher (getStatusCdr), with its CDR when SUNAT attaches it.
public struct VoucherStatus: Codable, Sendable, Hashable {
    public let statusCode: String
    public let statusMessage: String?
    public let cdr: CDR?
    public let cdrZip: Data?

    private enum CodingKeys: String, CodingKey {
        case statusCode = "status_code", statusMessage = "status_message", cdr, cdrZip = "cdr_zip_base64"
    }
}

/// What `status` got: the voucher's status, a SOAP fault, or no verdict (ask again).
public struct StatusResult: Codable, Sendable, Hashable, RawJSONCarrying {
    public enum Outcome: Sendable, Hashable {
        case status(VoucherStatus)
        case fault(Fault)
        case noVerdict(NoVerdict)
    }

    public let outcome: Outcome
    /// The input field behind each code SUNAT answered; empty when the response carries none.
    public let diagnosis: [Diagnosis]
    public internal(set) var rawJSON: String = ""

    private enum CodingKeys: String, CodingKey {
        case outcome, diagnosis
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        diagnosis = try container.decodeIfPresent([Diagnosis].self, forKey: .diagnosis) ?? []
        switch try container.decode(String.self, forKey: .outcome) {
        case "status": outcome = .status(try VoucherStatus(from: decoder))
        case "fault": outcome = .fault(try Fault(from: decoder))
        case "no_verdict": outcome = .noVerdict(try NoVerdict(from: decoder))
        case let other: throw unknownOutcome(other, container, CodingKeys.outcome)
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        if !diagnosis.isEmpty { try container.encode(diagnosis, forKey: .diagnosis) }
        switch outcome {
        case let .status(status):
            try container.encode("status", forKey: .outcome)
            try status.encode(to: encoder)
        case let .fault(fault):
            try container.encode("fault", forKey: .outcome)
            try fault.encode(to: encoder)
        case let .noVerdict(noVerdict):
            try container.encode("no_verdict", forKey: .outcome)
            try noVerdict.encode(to: encoder)
        }
    }
}

func unknownOutcome<K: CodingKey>(_ value: String, _ container: KeyedDecodingContainer<K>, _ key: K) -> DecodingError {
    DecodingError.dataCorruptedError(forKey: key, in: container, debugDescription: "unknown outcome `\(value)`")
}
