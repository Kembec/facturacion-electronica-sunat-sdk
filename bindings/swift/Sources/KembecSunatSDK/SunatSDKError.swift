/// Why a command of ``SunatSDKClient`` did not give a result.
///
/// A SOAP fault, a refused query or a missing answer to a query are results, not errors: they come back as an outcome
/// of the command (``SendResult/Outcome/fault(_:)``, ``NoVerdict``...).
public enum SunatSDKError: Error, Equatable, Sendable, CustomStringConvertible {
    /// The command ran and failed: the SDK rejected the document (the message says which field and why), the
    /// certificate could not be read, the endpoint is not `https://`, a send got no answer (it may have arrived: send the
    /// same XML again, never another number)... It is the CLI's `{"error"}` with exit code 1.
    case commandFailed(command: String, message: String)
    /// An argument was refused before any call: a document that is not one JSON value, a timeout out of range, a date
    /// without year, month or day.
    case invalidArgument(String)
    /// The native library is not the one this package was built for (another ABI or another version).
    case loadFailed(String)
    /// The library answered something that is not the contract's shape for the command.
    case unexpectedResponse(command: String, message: String)

    public var description: String {
        switch self {
        case let .commandFailed(command, message): return "\(command): \(message)"
        case let .invalidArgument(message): return message
        case let .loadFailed(message): return message
        case let .unexpectedResponse(command, message): return "\(command): unexpected response: \(message)"
        }
    }

    /// The command that failed; nil for errors raised before any call.
    public var command: String? {
        switch self {
        case let .commandFailed(command, _), let .unexpectedResponse(command, _): return command
        case .invalidArgument, .loadFailed: return nil
        }
    }
}
