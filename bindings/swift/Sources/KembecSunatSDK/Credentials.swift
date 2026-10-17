import Foundation

/// The certificate and its private key, both PEM files; the key unencrypted RSA. The SDK reads them on each `sign`.
public struct SigningKey: Sendable, Hashable {
    public let certificatePath: String
    public let privateKeyPath: String

    public init(certificatePath: String, privateKeyPath: String) {
        self.certificatePath = certificatePath
        self.privateKeyPath = privateKeyPath
    }

    /// From file URLs.
    public init(certificate: URL, privateKey: URL) {
        self.init(certificatePath: certificate.path, privateKeyPath: privateKey.path)
    }
}

/// The SOL user (the RUC followed by the user, `20123456789MODDATOS`) and its password. The password never shows in
/// `description`, `debugDescription` or `dump`.
public struct SOLCredentials: Sendable, Hashable, CustomStringConvertible, CustomDebugStringConvertible, CustomReflectable {
    public let user: String
    public let password: String

    public init(user: String, password: String) {
        self.user = user
        self.password = password
    }

    public var description: String { "SOLCredentials(user: \(user), password: ***)" }
    public var debugDescription: String { description }
    public var customMirror: Mirror { Mirror(self, children: ["user": user, "password": "***"]) }
}

/// A pair of «Credenciales de API SUNAT» created in SOL: the Consulta Integrada pair for `validity`, the GRE pair for
/// `greSend` and `greTicket`. The secret never shows in `description`, `debugDescription` or `dump`.
public struct APICredentials: Sendable, Hashable, CustomStringConvertible, CustomDebugStringConvertible, CustomReflectable {
    public let clientID: String
    public let clientSecret: String

    public init(clientID: String, clientSecret: String) {
        self.clientID = clientID
        self.clientSecret = clientSecret
    }

    public var description: String { "APICredentials(clientID: \(clientID), clientSecret: ***)" }
    public var debugDescription: String { description }
    public var customMirror: Mirror { Mirror(self, children: ["clientID": clientID, "clientSecret": "***"]) }
}
