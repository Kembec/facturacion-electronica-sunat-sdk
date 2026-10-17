import Foundation

// The results of the REST commands: Consulta Integrada (validity) and the despatch advice (gre-send, gre-ticket).

/// A voucher's state in the Consulta Integrada.
public enum ValidityState: String, Codable, Sendable, CaseIterable, WireEnum {
    /// 0. It does not prove the voucher never arrived: reconcile with `status` or the summary's ticket first.
    case notReported = "not_reported"
    /// 1
    case accepted
    /// 2: the number is spent.
    case voided
    /// 3
    case printedAuthorized = "printed_authorized"
    /// 4
    case printedNotAuthorized = "printed_not_authorized"
    case unknown

    public init(from decoder: Decoder) throws { self = try Self.decodeWire(decoder) }
}

/// What the Consulta Integrada says of a voucher.
public struct Validity: Codable, Sendable, Hashable {
    public let stateCode: String?
    public let state: ValidityState
    public let taxpayerStateCode: String?
    public let domicileConditionCode: String?
    /// Where the data sent differs from SUNAT's (an accepted voucher with another issue date, for instance).
    public let observations: [String]
    public let message: String?

    private enum CodingKeys: String, CodingKey {
        case stateCode = "state_code", state, taxpayerStateCode = "taxpayer_state_code"
        case domicileConditionCode = "domicile_condition_code", observations, message
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        stateCode = try container.decodeIfPresent(String.self, forKey: .stateCode)
        state = try container.decode(ValidityState.self, forKey: .state)
        taxpayerStateCode = try container.decodeIfPresent(String.self, forKey: .taxpayerStateCode)
        domicileConditionCode = try container.decodeIfPresent(String.self, forKey: .domicileConditionCode)
        observations = try container.decodeIfPresent([String].self, forKey: .observations) ?? []
        message = try container.decodeIfPresent(String.self, forKey: .message)
    }
}

/// SUNAT refused the query (not the voucher): `success: false` with HTTP 200, 400 or 422.
public struct QueryError: Codable, Sendable, Hashable {
    public let httpStatus: Int
    /// `99` is a malformed query.
    public let errorCode: String?
    public let message: String

    private enum CodingKeys: String, CodingKey {
        case httpStatus = "http_status", errorCode = "error_code", message
    }
}

/// What `validity` got: the voucher's state, a refused query, or no verdict (ask again).
public struct ValidityResult: Codable, Sendable, Hashable, RawJSONCarrying {
    public enum Outcome: Sendable, Hashable {
        case validity(Validity)
        case queryError(QueryError)
        case noVerdict(NoVerdict)
    }

    public let outcome: Outcome
    public internal(set) var rawJSON: String = ""

    private enum CodingKeys: String, CodingKey {
        case outcome
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        switch try container.decode(String.self, forKey: .outcome) {
        case "validity": outcome = .validity(try Validity(from: decoder))
        case "query_error": outcome = .queryError(try QueryError(from: decoder))
        case "no_verdict": outcome = .noVerdict(try NoVerdict(from: decoder))
        case let other: throw unknownOutcome(other, container, CodingKeys.outcome)
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch outcome {
        case let .validity(validity):
            try container.encode("validity", forKey: .outcome)
            try validity.encode(to: encoder)
        case let .queryError(error):
            try container.encode("query_error", forKey: .outcome)
            try error.encode(to: encoder)
        case let .noVerdict(noVerdict):
            try container.encode("no_verdict", forKey: .outcome)
            try noVerdict.encode(to: encoder)
        }
    }
}

/// One validation error of the GRE API.
public struct GREAPIErrorItem: Codable, Sendable, Hashable {
    public let code: String?
    public let description: String
}

/// SUNAT answered the GRE API's error body.
public struct GREAPIError: Codable, Sendable, Hashable {
    public let stage: ExchangeStage
    public let httpStatus: Int
    /// Annex I's status: `unauthorized`, `unprocessable_entity`...
    public let generalError: String?
    public let code: String?
    public let message: String
    /// What SUNAT rejected, in its words (422).
    public let errors: [GREAPIErrorItem]

    private enum CodingKeys: String, CodingKey {
        case stage, httpStatus = "http_status", generalError = "general_error", code, message, errors
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        stage = try container.decode(ExchangeStage.self, forKey: .stage)
        httpStatus = try container.decode(Int.self, forKey: .httpStatus)
        generalError = try container.decodeIfPresent(String.self, forKey: .generalError)
        code = try container.decodeIfPresent(String.self, forKey: .code)
        message = try container.decode(String.self, forKey: .message)
        errors = try container.decodeIfPresent([GREAPIErrorItem].self, forKey: .errors) ?? []
    }
}

/// The ticket of a sent guide: store it before anything else, it is the only handle on the outcome.
public struct GRETicket: Codable, Sendable, Hashable {
    public let ticket: String
    public let receivedAt: String?
    public let zipName: String?
    /// SHA-256 of the zip sent, lowercase hex.
    public let hashZip: String?

    private enum CodingKeys: String, CodingKey {
        case ticket, receivedAt = "received_at", zipName = "zip_name", hashZip = "hash_zip"
    }
}

/// What `greSend` got: the ticket, an error of the API, or no verdict on the token (nothing was sent). A send with no
/// answer is not here: it throws, since the guide may have arrived.
public struct GRESendResult: Codable, Sendable, Hashable, RawJSONCarrying {
    public enum Outcome: Sendable, Hashable {
        case ticket(GRETicket)
        case apiError(GREAPIError)
        case noVerdict(NoVerdict)
    }

    public let outcome: Outcome
    public internal(set) var rawJSON: String = ""

    private enum CodingKeys: String, CodingKey {
        case outcome
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        switch try container.decode(String.self, forKey: .outcome) {
        case "ticket": outcome = .ticket(try GRETicket(from: decoder))
        case "api_error": outcome = .apiError(try GREAPIError(from: decoder))
        case "no_verdict": outcome = .noVerdict(try NoVerdict(from: decoder))
        case let other: throw unknownOutcome(other, container, CodingKeys.outcome)
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch outcome {
        case let .ticket(ticket):
            try container.encode("ticket", forKey: .outcome)
            try ticket.encode(to: encoder)
        case let .apiError(error):
            try container.encode("api_error", forKey: .outcome)
            try error.encode(to: encoder)
        case let .noVerdict(noVerdict):
            try container.encode("no_verdict", forKey: .outcome)
            try noVerdict.encode(to: encoder)
        }
    }
}

/// The error of a guide processed with errors (99): SUNAT's `numError` and `desError`.
public struct GRETicketError: Codable, Sendable, Hashable {
    public let code: String?
    public let description: String
    public let responseClass: ResponseClass

    private enum CodingKeys: String, CodingKey {
        case code, description, responseClass = "class"
    }
}

/// The state of a sent guide.
public struct GRETicketStatus: Codable, Sendable, Hashable {
    public let statusCode: String
    public let state: TicketState
    public let cdrGenerated: Bool
    public let error: GRETicketError?
    public let cdr: CDR?
    public let cdrZip: Data?

    private enum CodingKeys: String, CodingKey {
        case statusCode = "status_code", state, cdrGenerated = "cdr_generated", error, cdr, cdrZip = "cdr_zip_base64"
    }
}

/// What `greTicket` got: the guide's state, an error of the API, or no verdict (ask again with the same ticket).
public struct GRETicketResult: Codable, Sendable, Hashable, RawJSONCarrying {
    public enum Outcome: Sendable, Hashable {
        case ticketStatus(GRETicketStatus)
        case apiError(GREAPIError)
        case noVerdict(NoVerdict)
    }

    public let outcome: Outcome
    /// The ticket asked for.
    public let ticket: String
    public internal(set) var rawJSON: String = ""

    private enum CodingKeys: String, CodingKey {
        case outcome, ticket
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        ticket = try container.decode(String.self, forKey: .ticket)
        switch try container.decode(String.self, forKey: .outcome) {
        case "ticket_status": outcome = .ticketStatus(try GRETicketStatus(from: decoder))
        case "api_error": outcome = .apiError(try GREAPIError(from: decoder))
        case "no_verdict": outcome = .noVerdict(try NoVerdict(from: decoder))
        case let other: throw unknownOutcome(other, container, CodingKeys.outcome)
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encode(ticket, forKey: .ticket)
        switch outcome {
        case let .ticketStatus(status):
            try container.encode("ticket_status", forKey: .outcome)
            try status.encode(to: encoder)
        case let .apiError(error):
            try container.encode("api_error", forKey: .outcome)
            try error.encode(to: encoder)
        case let .noVerdict(noVerdict):
            try container.encode("no_verdict", forKey: .outcome)
            try noVerdict.encode(to: encoder)
        }
    }
}
