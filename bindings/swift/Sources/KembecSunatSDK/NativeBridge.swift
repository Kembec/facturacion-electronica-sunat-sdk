import Foundation
import SunatSDKFFI

/// One call of the C ABI (docs/c-abi.md): the request written, the call, the response read, released and decoded.
enum NativeBridge {
    static let abiVersion: UInt32 = 1

    /// Checked once per process: a library of another ABI or another version is refused before its first command.
    static let loaded: Result<String, SunatSDKError> = {
        let abi = sunat_sdk_abi_version()
        guard abi == abiVersion else {
            return .failure(.loadFailed("the native library has ABI \(abi); this package needs \(abiVersion)"))
        }
        guard let pointer = sunat_sdk_version() else {
            return .failure(.loadFailed("the native library gives no version"))
        }
        let version = String(cString: pointer)
        guard version == SunatSDKClient.packageVersion else {
            return .failure(.loadFailed("the native library is \(version); this package is \(SunatSDKClient.packageVersion)"))
        }
        return .success(version)
    }()

    /// Runs `command` and decodes its response as `T`. Status 1 is ``SunatSDKError/commandFailed(command:message:)``.
    static func run<T: RawJSONCarrying>(_ command: String, flags: [(String, RequestValue)] = [], input: (inout RequestWriter) -> Void, as type: T.Type) throws -> T {
        _ = try loaded.get()
        var writer = RequestWriter()
        writer.raw("{\"command\":")
        writer.string(command)
        writer.raw(",\"flags\":")
        writer.object(flags)
        writer.raw(",\"input\":")
        input(&writer)
        writer.raw("}")
        let (status, text) = writer.call()
        guard let text else {
            throw SunatSDKError.unexpectedResponse(command: command, message: "no response, or one that is not UTF-8 (status \(status))")
        }
        let data = Data(text.utf8)
        switch status {
        case 0:
            do {
                var value = try JSONDecoder().decode(T.self, from: data)
                value.rawJSON = text
                return value
            } catch let error as DecodingError {
                throw SunatSDKError.unexpectedResponse(command: command, message: describe(error))
            }
        case 1:
            guard let body = try? JSONDecoder().decode(ErrorBody.self, from: data) else {
                throw SunatSDKError.unexpectedResponse(command: command, message: "status 1 without {\"error\"}")
            }
            throw SunatSDKError.commandFailed(command: command, message: body.error)
        default:
            throw SunatSDKError.unexpectedResponse(command: command, message: "status \(status)")
        }
    }

    private struct ErrorBody: Decodable {
        let error: String
    }

    private static func describe(_ error: DecodingError) -> String {
        switch error {
        case let .keyNotFound(key, context):
            return "`\(path(context.codingPath + [key]))` is missing"
        case let .typeMismatch(_, context), let .valueNotFound(_, context), let .dataCorrupted(context):
            return "`\(path(context.codingPath))`: \(context.debugDescription)"
        @unknown default:
            return "\(error)"
        }
    }

    private static func path(_ keys: [CodingKey]) -> String {
        keys.map { $0.intValue.map(String.init) ?? $0.stringValue }.joined(separator: ".")
    }
}

/// A value of the request: text, a whole number, or JSON text passed as written (`build`'s document).
enum RequestValue {
    case text(String)
    case number(Int64)
    case json(String)
}

/// The request as UTF-8 bytes, written by hand so the document reaches the command as the caller wrote it and the
/// secrets live in one buffer that is overwritten after the call.
struct RequestWriter {
    private var bytes: [UInt8] = []

    init() {
        bytes.reserveCapacity(4096)
    }

    mutating func raw(_ text: String) {
        bytes.append(contentsOf: text.utf8)
    }

    mutating func string(_ text: String) {
        bytes.append(0x22)
        for unit in text.utf8 {
            switch unit {
            case 0x22: bytes.append(contentsOf: [0x5C, 0x22])
            case 0x5C: bytes.append(contentsOf: [0x5C, 0x5C])
            case 0x00..<0x20:
                let hex = Array("0123456789abcdef".utf8)
                bytes.append(contentsOf: [0x5C, 0x75, 0x30, 0x30, hex[Int(unit >> 4)], hex[Int(unit & 0xF)]])
            default: bytes.append(unit)
            }
        }
        bytes.append(0x22)
    }

    mutating func value(_ value: RequestValue) {
        switch value {
        case let .text(text): string(text)
        case let .number(number): raw(String(number))
        case let .json(text): raw(text)
        }
    }

    /// An object of the given members, in order.
    mutating func object(_ members: [(String, RequestValue)]) {
        raw("{")
        for (index, member) in members.enumerated() {
            if index > 0 { raw(",") }
            string(member.0)
            raw(":")
            value(member.1)
        }
        raw("}")
    }

    /// Calls the library with the request, then overwrites the request's bytes. Returns the status and the response.
    mutating func call() -> (Int32, String?) {
        bytes.append(0)
        defer {
            bytes.withUnsafeMutableBytes { buffer in
                if let base = buffer.baseAddress { _ = memset(base, 0, buffer.count) }
            }
        }
        var response: UnsafeMutablePointer<CChar>?
        let status = bytes.withUnsafeBufferPointer { buffer -> Int32 in
            buffer.withMemoryRebound(to: CChar.self) { sunat_sdk_call($0.baseAddress, &response) }
        }
        guard let response else { return (status, nil) }
        defer { sunat_sdk_free(response) }
        return (status, String(validatingUTF8: response))
    }
}

/// Results that keep the response as the library gave it.
protocol RawJSONCarrying: Decodable {
    var rawJSON: String { get set }
}
