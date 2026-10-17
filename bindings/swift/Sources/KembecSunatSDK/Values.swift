import Foundation

// The documents and the values inside the network outcomes. Coding keys are the contract's (docs/cli.md), so encoding
// a value gives the same JSON as every other language. `rawJSON` is not encoded: it is the library's response, empty
// for a value decoded from anywhere else or nested in another result.

/// The unsigned UBL XML of `build`, and the file name SUNAT expects (without extension).
public struct BuildResult: Codable, Sendable, Hashable, RawJSONCarrying {
    /// `20123456789-01-F001-1`, `20123456789-RC-20261016-1`...
    public let fileName: String
    public let xml: String
    public internal(set) var rawJSON: String = ""

    private enum CodingKeys: String, CodingKey {
        case fileName = "file_name", xml
    }
}

/// The signed XML of `sign`, with the two values a signature is compared by.
public struct SignResult: Codable, Sendable, Hashable, RawJSONCarrying {
    public let xml: String
    public let digestValue: String
    public let signatureValue: String
    public internal(set) var rawJSON: String = ""

    private enum CodingKeys: String, CodingKey {
        case xml, digestValue = "digest_value", signatureValue = "signature_value"
    }
}

/// The zip of `zip`, with the XML inside under `<name>.xml`.
public struct ZipResult: Codable, Sendable, Hashable, RawJSONCarrying {
    /// `<name>.zip`
    public let fileName: String
    public let content: Data
    public internal(set) var rawJSON: String = ""

    private enum CodingKeys: String, CodingKey {
        case fileName = "file_name", content = "zip_base64"
    }
}

/// A CDR (constancia de recepción): the code SUNAT answered and how the SDK classifies it.
public struct CDR: Codable, Sendable, Hashable, RawJSONCarrying {
    /// The voucher it answers: `F001-1`.
    public let referenceID: String
    /// `0`, `2047`...
    public let responseCode: String
    public let description: String
    /// Each `code - text`; with code 0, every note is an observation.
    public let notes: [String]
    public let responseClass: ResponseClass
    public internal(set) var rawJSON: String = ""

    /// Code 0 without observations.
    public var isAccepted: Bool { responseClass == .accepted }
    /// Accepted with or without observations: the voucher is valid.
    public var isValidDocument: Bool { responseClass == .accepted || responseClass == .observed }

    private enum CodingKeys: String, CodingKey {
        case referenceID = "reference_id", responseCode = "response_code", description, notes, responseClass = "class"
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        referenceID = try container.decode(String.self, forKey: .referenceID)
        responseCode = try container.decode(String.self, forKey: .responseCode)
        description = try container.decode(String.self, forKey: .description)
        notes = try container.decodeIfPresent([String].self, forKey: .notes) ?? []
        responseClass = try container.decode(ResponseClass.self, forKey: .responseClass)
    }
}

/// A SOAP fault: SUNAT refused the call with a code.
public struct Fault: Codable, Sendable, Hashable {
    /// `1033`, `0111`...
    public let code: String
    public let message: String
    /// `.alreadyRegistered` for 1033, and so on.
    public let responseClass: ResponseClass

    private enum CodingKeys: String, CodingKey {
        case code, message, responseClass = "class"
    }
}

/// A query without a verdict: SUNAT did not answer in time or answered an HTTP status outside the contract. It says
/// nothing about the document: ask again, never send again.
public struct NoVerdict: Codable, Sendable, Hashable {
    public let reason: NoVerdictReason
    /// The HTTP status SUNAT answered; nil when the wait ran out.
    public let httpStatus: Int?
    public let message: String
    /// The exchange that got no verdict (`validity`, `greSend`, `greTicket`); nil for `status` and `ticket`.
    public let stage: ExchangeStage?

    private enum CodingKeys: String, CodingKey {
        case reason, httpStatus = "http_status", message, stage
    }
}

/// How the SDK classifies a SUNAT code.
public enum ResponseClass: String, Codable, Sendable, CaseIterable, WireEnum {
    case accepted
    case observed
    case exception
    case alreadyRegistered = "already_registered"
    case numberVoidedOrRejected = "number_voided_or_rejected"
    case rejected
    case voidAlreadyReported = "void_already_reported"
    /// A code the SDK does not classify, or none.
    case unknown

    public init(from decoder: Decoder) throws { self = try Self.decodeWire(decoder) }
}

/// The state of a ticket (`ticket`, `greTicket`).
public enum TicketState: String, Codable, Sendable, CaseIterable, WireEnum {
    /// 0
    case processed
    /// 98: ask again with the same ticket, never send again.
    case inProcess = "in_process"
    /// 99
    case processedWithErrors = "processed_with_errors"
    case unknown

    public init(from decoder: Decoder) throws { self = try Self.decodeWire(decoder) }
}

/// Why a query has no verdict.
public enum NoVerdictReason: String, Codable, Sendable, CaseIterable, WireEnum {
    case timeout
    case httpStatus = "http_status"
    case unknown

    public init(from decoder: Decoder) throws { self = try Self.decodeWire(decoder) }
}

/// The exchange of a REST command that a result speaks of.
public enum ExchangeStage: String, Codable, Sendable, CaseIterable, WireEnum {
    case token
    case query
    case send
    case ticket
    case unknown

    public init(from decoder: Decoder) throws { self = try Self.decodeWire(decoder) }
}

/// The contract's enumerations: a value this version does not know, or null, is `.unknown` rather than an error.
protocol WireEnum: RawRepresentable where RawValue == String {
    static var unknown: Self { get }
}

extension WireEnum {
    static func decodeWire(_ decoder: Decoder) throws -> Self {
        let container = try decoder.singleValueContainer()
        if container.decodeNil() { return .unknown }
        return Self(rawValue: try container.decode(String.self)) ?? .unknown
    }
}

extension KeyedDecodingContainer {
    // A missing member of a contract enumeration is `.unknown` too; synthesized decoders pick this overload.
    func decode<T: WireEnum & Decodable>(_ type: T.Type, forKey key: Key) throws -> T {
        try decodeIfPresent(type, forKey: key) ?? .unknown
    }
}
