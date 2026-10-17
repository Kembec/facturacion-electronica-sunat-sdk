import Foundation

// Which field of the input JSON a SUNAT rejection or observation points at (docs/cli.md#diagnosis). The core writes it
// into the responses of send, summary, ticket and status; the results read it as `diagnosis`, empty when absent.

/// What SUNAT's message names, as it writes it: `(nodo: "cbc:ID/schemeID" valor: "1")`.
public struct DiagnosisDetail: Codable, Sendable, Hashable {
    /// The last element and, if any, its attribute: `cbc:ID/schemeID`.
    public let node: String
    /// The value SUNAT read in that node.
    public let value: String
}

/// How sure the table is of a field.
public enum DiagnosisCertainty: Sendable, Hashable {
    /// `fuente`: the node is the rule's own and one field writes it.
    case source
    /// `inferido`: the rule names more than the node, or the field is a reading of it.
    case inferred
    /// A word the contract may add later.
    case other(String)
}

extension DiagnosisCertainty: Codable {
    public init(from decoder: Decoder) throws {
        switch try decoder.singleValueContainer().decode(String.self) {
        case "fuente": self = .source
        case "inferido": self = .inferred
        case let other: self = .other(other)
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()
        switch self {
        case .source: try container.encode("fuente")
        case .inferred: try container.encode("inferido")
        case let .other(word): try container.encode(word)
        }
    }
}

/// One candidate field for a code, from SUNAT's validation rules of 26.08.2026.
public struct DiagnosisField: Codable, Sendable, Hashable {
    /// A field of the input model (`customer.document_type`, `lines[].unit_price`), `sdk` when the SDK writes the node
    /// itself, or nil when the input model has no such field.
    public let inputPath: String?
    /// The node the rule checks; nil for rules on the file, the signature or the service.
    public let xpath: String?
    public let certainty: DiagnosisCertainty

    private enum CodingKeys: String, CodingKey {
        case inputPath = "input_path", xpath, certainty
    }
}

/// A code SUNAT answered, with the fields that may have caused it.
public struct Diagnosis: Codable, Sendable, Hashable {
    /// `2800`, `4287`...
    public let code: String
    /// What the message names, when it names a node.
    public let detail: DiagnosisDetail?
    /// With a `detail`, the fields that write that node, unless none does; otherwise all of the code's. Empty when the
    /// code is not in the table.
    public let fields: [DiagnosisField]

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        code = try container.decode(String.self, forKey: .code)
        detail = try container.decodeIfPresent(DiagnosisDetail.self, forKey: .detail)
        fields = try container.decodeIfPresent([DiagnosisField].self, forKey: .fields) ?? []
    }

    private enum CodingKeys: String, CodingKey {
        case code, detail, fields
    }
}
