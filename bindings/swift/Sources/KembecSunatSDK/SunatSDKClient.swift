import Foundation

/// The commands of the SDK's contract (docs/cli.md) in process, through its C ABI (docs/c-abi.md): the same validation,
/// XML, signature, network calls and outcomes as the CLI and every other language.
///
/// A value without state, safe to share between threads and tasks. Each method blocks until its command ends; the
/// network ones wait on SUNAT up to ``httpTimeout``. From Swift concurrency use the `async` overloads
/// (`try await client.send(...)`), which run the call on a dispatch queue and leave the caller's thread free.
public struct SunatSDKClient: Sendable {
    /// The version of this package, equal to the native library's.
    public static let packageVersion = "1.0.0-rc.5"
    /// 90 seconds, the CLI's default.
    public static let defaultHTTPTimeout: TimeInterval = 90

    /// How long a network command waits on SUNAT: the token and the call of the REST commands share it.
    public let httpTimeout: TimeInterval
    private let timeoutSeconds: Int64

    /// A client that waits on SUNAT up to `httpTimeout`.
    /// - Throws: ``SunatSDKError/invalidArgument(_:)`` unless it is whole seconds from 1 to 300.
    public init(httpTimeout: TimeInterval = SunatSDKClient.defaultHTTPTimeout) throws {
        guard httpTimeout.rounded() == httpTimeout, httpTimeout >= 1, httpTimeout <= 300 else {
            throw SunatSDKError.invalidArgument("httpTimeout must be whole seconds from 1 to 300, not \(httpTimeout)")
        }
        self.httpTimeout = httpTimeout
        timeoutSeconds = Int64(httpTimeout)
    }

    /// The version of the native library linked, which is the SDK's.
    /// - Throws: ``SunatSDKError/loadFailed(_:)`` if the library is not the one this package was built for.
    public static func nativeVersion() throws -> String {
        try NativeBridge.loaded.get()
    }

    /// Renders the unsigned UBL XML of one voucher, daily summary, void communication or despatch advice.
    /// - Parameter documentJSON: the document as JSON, in the shape of docs/input-model.md.
    /// - Throws: ``SunatSDKError/invalidArgument(_:)`` if the text is not one JSON value;
    ///   ``SunatSDKError/commandFailed(command:message:)`` if the SDK rejected the document (the message says which
    ///   field and why).
    public func build(documentJSON: String) throws -> BuildResult {
        do {
            _ = try JSONSerialization.jsonObject(with: Data(documentJSON.utf8), options: [.fragmentsAllowed])
        } catch {
            throw SunatSDKError.invalidArgument("documentJSON is not one JSON value: \(error.localizedDescription)")
        }
        return try NativeBridge.run("build", input: { $0.raw(documentJSON) }, as: BuildResult.self)
    }

    /// Renders a document given as any `Encodable` value whose JSON has the shape of docs/input-model.md.
    public func build<Document: Encodable>(_ document: Document) throws -> BuildResult {
        let data = try JSONEncoder().encode(document)
        return try build(documentJSON: String(decoding: data, as: UTF8.self))
    }

    /// Signs XML from ``build(documentJSON:)`` with XML-DSig (enveloped, RSA-SHA1, as SUNAT requires).
    /// - Parameter signatureID: the signature's `Id`; `signatureKG` when nil.
    public func sign(xml unsignedXML: String, key: SigningKey, signatureID: String? = nil) throws -> SignResult {
        let flags: [(String, RequestValue)] = [("cert", .text(key.certificatePath)), ("key", .text(key.privateKeyPath))]
        var input: [(String, RequestValue)] = [("xml", .text(unsignedXML))]
        if let signatureID { input.append(("signature_id", .text(signatureID))) }
        return try NativeBridge.run("sign", flags: flags, input: { $0.object(input) }, as: SignResult.self)
    }

    /// Zips the XML under `<name>.xml`, as SUNAT expects it.
    /// - Parameter name: the file name without extension (`20100066603-01-F001-1`).
    public func zip(name: String, xml: String) throws -> ZipResult {
        try NativeBridge.run("zip", input: { $0.object([("name", .text(name)), ("xml", .text(xml))]) }, as: ZipResult.self)
    }

    /// Reads a CDR, zip or XML, and classifies its response code.
    public func readCDR(_ cdr: Data) throws -> CDR {
        let encoded = cdr.base64EncodedString()
        return try NativeBridge.run("cdr", input: { $0.object([("cdr_base64", .text(encoded))]) }, as: CDR.self)
    }

    /// Zips the signed XML and sends it with sendBill. A fault is a result; a send without an answer throws
    /// ``SunatSDKError/commandFailed(command:message:)``, since it may have arrived: send the same XML again, never
    /// another number.
    /// - Parameter endpoint: the billService URL, https only.
    public func send(name: String, signedXML: String, credentials: SOLCredentials, endpoint: URL) throws -> SendResult {
        try NativeBridge.run("send", flags: soap(credentials, endpoint), input: { $0.object([("name", .text(name)), ("xml", .text(signedXML))]) },
                             as: SendResult.self)
    }

    /// Zips a signed daily summary (RC) or void communication (RA) and sends it with sendSummary. Store the ticket
    /// before anything else. A send without an answer throws, as ``send(name:signedXML:credentials:endpoint:)``.
    /// - Parameter name: `RUC-RC|RA-YYYYMMDD-N`.
    public func summary(name: String, signedXML: String, credentials: SOLCredentials, endpoint: URL) throws -> SummaryResult {
        try NativeBridge.run("summary", flags: soap(credentials, endpoint), input: { $0.object([("name", .text(name)), ("xml", .text(signedXML))]) },
                             as: SummaryResult.self)
    }

    /// Asks getStatus for a summary's or a void's ticket. No answer is `.noVerdict`: ask again, never send again.
    public func ticket(_ ticket: String, credentials: SOLCredentials, endpoint: URL) throws -> TicketResult {
        try NativeBridge.run("ticket", flags: soap(credentials, endpoint), input: { $0.object([("ticket", .text(ticket))]) }, as: TicketResult.self)
    }

    /// Asks getStatusCdr for one voucher, with its CDR when SUNAT attaches it. No answer is `.noVerdict`.
    /// - Parameters:
    ///   - documentType: `01`, `03`, `07` or `08`.
    ///   - endpoint: the billConsultService URL, https only.
    public func status(ruc: String, documentType: String, series: String, number: Int64, credentials: SOLCredentials,
                       endpoint: URL) throws -> StatusResult {
        try NativeBridge.run("status", flags: soap(credentials, endpoint),
                             input: { $0.object(voucherKey(ruc, documentType, series, number)) }, as: StatusResult.self)
    }

    /// Asks SUNAT's Consulta Integrada whether a voucher (01, 03, 07, 08) exists and in which state: a token, then the
    /// query, both inside ``httpTimeout``. There is no test environment: this is production.
    /// - Parameters:
    ///   - issueDate: the issue date; year, month and day are required, the rest is ignored.
    ///   - total: the voucher's total (PayableAmount), up to 2 decimals.
    ///   - credentials: the Consulta Integrada pair of «Credenciales de API SUNAT».
    ///   - requesterRUC: the RUC that asks; the issuer's when nil.
    ///   - tokenEndpoint: replaces the production token base (https only); nil keeps it.
    ///   - endpoint: replaces the production query base (https only); nil keeps it.
    public func validity(ruc: String, documentType: String, series: String, number: Int64, issueDate: DateComponents,
                         total: Decimal, credentials: APICredentials, requesterRUC: String? = nil, tokenEndpoint: URL? = nil,
                         endpoint: URL? = nil) throws -> ValidityResult {
        guard let year = issueDate.year, let month = issueDate.month, let day = issueDate.day,
              (1...9999).contains(year), (1...12).contains(month), (1...31).contains(day) else {
            throw SunatSDKError.invalidArgument("issueDate needs a year, a month and a day")
        }
        var flags: [(String, RequestValue)] = [("client-id", .text(credentials.clientID)), ("client-secret", .text(credentials.clientSecret)),
                                               ("timeout", .number(timeoutSeconds))]
        if let tokenEndpoint { flags.append(("token-endpoint", .text(tokenEndpoint.absoluteString))) }
        if let endpoint { flags.append(("endpoint", .text(endpoint.absoluteString))) }
        var input = voucherKey(ruc, documentType, series, number)
        input.append(("issue_date", .text(String(format: "%04d-%02d-%02d", year, month, day))))
        // The decimal's own digits, never a binary double: 118.10 stays 118.1, not 118.09999999999999.
        input.append(("total", .text(NSDecimalNumber(decimal: total).description(withLocale: Locale(identifier: "en_US_POSIX")))))
        if let requesterRUC { input.append(("requester_ruc", .text(requesterRUC))) }
        return try NativeBridge.run("validity", flags: flags, input: { $0.object(input) }, as: ValidityResult.self)
    }

    /// Sends a signed despatch advice (09) by SUNAT's GRE REST API: a token by password, then the zip with its hash.
    /// SUNAT has no test environment for GRE and the endpoints have no default. A send without an answer throws: send
    /// the same file again, never the operation with another number.
    /// - Parameters:
    ///   - name: `RUC-09-SERIE-NUMERO`.
    ///   - api: the GRE pair of «Credenciales de API SUNAT».
    ///   - sol: the RUC followed by the SOL user, and its password.
    ///   - tokenEndpoint: the token base, https only (`https://api-seguridad.sunat.gob.pe/v1/clientessol`).
    ///   - endpoint: the API base, https only (`https://api-cpe.sunat.gob.pe/v1`).
    public func greSend(name: String, signedXML: String, api: APICredentials, sol: SOLCredentials, tokenEndpoint: URL,
                        endpoint: URL) throws -> GRESendResult {
        try NativeBridge.run("gre-send", flags: gre(api, sol, tokenEndpoint, endpoint),
                             input: { $0.object([("name", .text(name)), ("xml", .text(signedXML))]) }, as: GRESendResult.self)
    }

    /// Asks for the outcome of a sent guide by its ticket (a token, then the query). 98 is in process: ask again with
    /// the same ticket.
    public func greTicket(_ ticket: String, api: APICredentials, sol: SOLCredentials, tokenEndpoint: URL,
                          endpoint: URL) throws -> GRETicketResult {
        try NativeBridge.run("gre-ticket", flags: gre(api, sol, tokenEndpoint, endpoint), input: { $0.object([("ticket", .text(ticket))]) },
                             as: GRETicketResult.self)
    }

    private func soap(_ credentials: SOLCredentials, _ endpoint: URL) -> [(String, RequestValue)] {
        [("endpoint", .text(endpoint.absoluteString)), ("user", .text(credentials.user)), ("password", .text(credentials.password)),
         ("timeout", .number(timeoutSeconds))]
    }

    private func gre(_ api: APICredentials, _ sol: SOLCredentials, _ tokenEndpoint: URL, _ endpoint: URL) -> [(String, RequestValue)] {
        [("client-id", .text(api.clientID)), ("client-secret", .text(api.clientSecret)), ("user", .text(sol.user)),
         ("password", .text(sol.password)), ("token-endpoint", .text(tokenEndpoint.absoluteString)),
         ("endpoint", .text(endpoint.absoluteString)), ("timeout", .number(timeoutSeconds))]
    }

    private func voucherKey(_ ruc: String, _ documentType: String, _ series: String, _ number: Int64) -> [(String, RequestValue)] {
        [("ruc", .text(ruc)), ("document_type", .text(documentType)), ("series", .text(series)), ("number", .number(number))]
    }
}
