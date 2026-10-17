import CBatterySupport
import Dispatch
import Foundation
import KembecSunatSDK

/// The battery of the Swift package as a consumer links it: the 38 fixtures against their vectors (inclusive C14N,
/// exact DigestValue and SignatureValue), the native version against the package's, zip and CDR, the errors, Codable
/// round trips, no_verdict without an answer, concurrent calls and the async overloads.
public enum SwiftBattery {
    public static let expectedFixtures = 38

    public struct Report: Sendable {
        public let checks: Int
        public let failures: [String]
        public let vectors: Int
        /// The line a runner prints: `ok Swift: …` or `Swift: n of m checks failed, …`.
        public let summary: String
    }

    public static func run(repository: URL) -> Report {
        let checker = Checker()
        let repo = Repository(root: repository.standardizedFileURL)
        let key = SigningKey(certificate: repo.file("tests/vectors/test-cert/test-cert.pem"),
                             privateKey: repo.file("tests/vectors/test-cert/test-key.pem"))
        guard let client = try? SunatSDKClient(httpTimeout: 30) else {
            checker.check(false, "a client with a 30 s timeout")
            return report(checker, vectors: 0)
        }
        checker.run("version") {
            let native = try SunatSDKClient.nativeVersion()
            checker.check(native == SunatSDKClient.packageVersion, "native \(native), package \(SunatSDKClient.packageVersion)")
        }
        let fixtures = repo.fixtures()
        checker.check(fixtures.count == expectedFixtures, "fixtures: \(fixtures.count), expected \(expectedFixtures)")
        for fixture in fixtures {
            checker.run(fixture) { try vector(client, key, repo, fixture, checker) }
        }
        checker.run("zip") { try zip(client, key, repo, checker) }
        checker.run("cdr") { try readCDR(client, repo, checker) }
        checker.run("errors") { try errors(client, key, repo, checker) }
        checker.run("no_verdict") { try SilentChecks(checker: checker).run() }
        checker.run("threads") { try threads(client, key, repo, Array(fixtures.prefix(8)), checker) }
        return report(checker, vectors: fixtures.count)
    }

    private static func report(_ checker: Checker, vectors: Int) -> Report {
        let (checks, failures) = checker.totals()
        let native = (try? SunatSDKClient.nativeVersion()) ?? "not loaded"
        let summary = failures.isEmpty
            ? "ok Swift: \(checks) checks, \(vectors) vectors, \(platform()), native \(native)"
            : "Swift: \(failures.count) of \(checks) checks failed, \(platform())"
        return Report(checks: checks, failures: failures, vectors: vectors, summary: summary)
    }

    private static func vector(_ client: SunatSDKClient, _ key: SigningKey, _ repo: Repository, _ name: String, _ checker: Checker) throws {
        let document = try repo.text("fixtures/\(name).json")
        let built = try client.build(documentJSON: document)
        checker.check(sameCanonical(try repo.vector(name, "unsigned"), built.xml), "\(name): unsigned XML differs from its vector under C14N")
        checker.check(built.fileName == (try expectedFileName(document)), "\(name): file name \(built.fileName)")
        checker.check(!built.rawJSON.isEmpty, "\(name): rawJSON of build is empty")

        let signed = try client.sign(xml: built.xml, key: key)
        let expected = try repo.vector(name, "signed")
        checker.check(sameCanonical(expected, signed.xml), "\(name): signed XML differs from its vector under C14N")
        checker.check(expected.contains("<ds:DigestValue>\(signed.digestValue)</ds:DigestValue>"), "\(name): DigestValue \(signed.digestValue)")
        checker.check(expected.contains("<ds:SignatureValue>\(signed.signatureValue)</ds:SignatureValue>"), "\(name): SignatureValue differs")
        checker.check(signed.xml.contains("<ds:DigestValue>\(signed.digestValue)</ds:DigestValue>"), "\(name): DigestValue not in the signed XML")
    }

    private static func zip(_ client: SunatSDKClient, _ key: SigningKey, _ repo: Repository, _ checker: Checker) throws {
        let signed = try signFixture(client, key, repo, "01-invoice-taxed-cash")
        let zip = try client.zip(name: "20123456789-01-F001-1", xml: signed)
        checker.check(zip.fileName == "20123456789-01-F001-1.zip", "zip: file name \(zip.fileName)")
        checker.check(zip.content.count > 4 && zip.content.prefix(2) == Data("PK".utf8), "zip: not a zip")
    }

    private static func readCDR(_ client: SunatSDKClient, _ repo: Repository, _ checker: Checker) throws {
        let bytes = try Data(contentsOf: repo.file("crates/sunat-transport/tests/data/R-20150147718-01-FT01-982.zip"))
        let cdr = try client.readCDR(bytes)
        checker.check(cdr.referenceID == "FT01-982", "cdr: reference \(cdr.referenceID)")
        checker.check(cdr.responseCode == "2047", "cdr: code \(cdr.responseCode)")
        checker.check(cdr.responseClass == .rejected && !cdr.isValidDocument, "cdr: class \(cdr.responseClass)")
        let again = try JSONDecoder().decode(CDR.self, from: JSONEncoder().encode(cdr))
        checker.check(again.referenceID == cdr.referenceID && again.responseClass == cdr.responseClass && again.notes == cdr.notes,
                      "cdr: Codable round trip")
    }

    private static func errors(_ client: SunatSDKClient, _ key: SigningKey, _ repo: Repository, _ checker: Checker) throws {
        let build = checker.expectError("build of {} is the SDK's error") { _ = try client.build(documentJSON: "{}") }
        checker.check(build?.command == "build" && isCommandFailed(build), "build of {}: \(String(describing: build))")
        let json = checker.expectError("invalid JSON") { _ = try client.build(documentJSON: "{not json") }
        checker.check(isInvalidArgument(json, containing: "documentJSON"), "invalid JSON names the parameter: \(String(describing: json))")
        let timeout = checker.expectError("timeout of 1.5 s") { _ = try SunatSDKClient(httpTimeout: 1.5) }
        checker.check(isInvalidArgument(timeout, containing: "httpTimeout"), "timeout of 1.5 s: \(String(describing: timeout))")
        let date = checker.expectError("issue date without a day") {
            _ = try client.validity(ruc: "20123456789", documentType: "01", series: "F001", number: 1,
                                    issueDate: DateComponents(year: 2026, month: 10), total: 118,
                                    credentials: APICredentials(clientID: "id", clientSecret: "secret"))
        }
        checker.check(isInvalidArgument(date, containing: "issueDate"), "issue date without a day: \(String(describing: date))")

        let sol = SOLCredentials(user: "20123456789MODDATOS", password: "moddatos-secret")
        var dumped = ""
        dump(sol, to: &dumped)
        checker.check(!"\(sol)".contains("moddatos-secret") && !String(reflecting: sol).contains("moddatos-secret") && !dumped.contains("moddatos-secret"),
                      "SOLCredentials shows the password")
        let signed = try signFixture(client, key, repo, "01-invoice-taxed-cash")
        let http = checker.expectError("send over http://") {
            _ = try client.send(name: "20123456789-01-F001-1", signedXML: signed, credentials: sol, endpoint: URL(string: "http://127.0.0.1/billService")!)
        }
        if case let .commandFailed(command, message)? = http {
            checker.check(command == "send" && message.contains("endpoint must use https://") && !message.contains("moddatos-secret"),
                          "send over http://: \(message)")
        } else {
            checker.check(false, "send over http://: \(String(describing: http))")
        }
    }

    private static func threads(_ client: SunatSDKClient, _ key: SigningKey, _ repo: Repository, _ chosen: [String], _ checker: Checker) throws {
        let documents = try chosen.map { try repo.text("fixtures/\($0).json") }
        let expected = try chosen.map { try repo.vector($0, "signed") }
        let digests = Locked([String?](repeating: nil, count: chosen.count))
        DispatchQueue.concurrentPerform(iterations: chosen.count) { index in
            var digest: String?
            for _ in 0..<5 {
                digest = try? client.sign(xml: client.build(documentJSON: documents[index]).xml, key: key).digestValue
            }
            digests.update { $0[index] = digest }
        }
        let got = digests.value
        for index in chosen.indices {
            checker.check(got[index].map { expected[index].contains("<ds:DigestValue>\($0)</ds:DigestValue>") } ?? false, "threads: \(chosen[index])")
        }
    }

    static func signFixture(_ client: SunatSDKClient, _ key: SigningKey, _ repo: Repository, _ name: String) throws -> String {
        try client.sign(xml: client.build(documentJSON: repo.text("fixtures/\(name).json")).xml, key: key).xml
    }

    private static func isCommandFailed(_ error: SunatSDKError?) -> Bool {
        if case .commandFailed? = error { return true }
        return false
    }

    private static func isInvalidArgument(_ error: SunatSDKError?, containing text: String) -> Bool {
        if case let .invalidArgument(message)? = error { return message.contains(text) }
        return false
    }

    /// `ruc-type-series-number`, or `ruc-RC|RA-YYYYMMDD-number`, from the fixture's own fields, read with Foundation's
    /// parser and not the package's.
    static func expectedFileName(_ document: String) throws -> String {
        guard let object = try JSONSerialization.jsonObject(with: Data(document.utf8)) as? [String: Any],
              let issuer = object["issuer"] as? [String: Any] else { return "" }
        let field = { (value: Any?) in value.map { "\($0)" } ?? "" }
        let ruc = field(issuer["ruc"]), number = field(object["number"])
        switch field(object["kind"]) {
        case "summary": return "\(ruc)-RC-\(field(object["generation_date"]).replacingOccurrences(of: "-", with: ""))-\(number)"
        case "voided": return "\(ruc)-RA-\(field(object["communication_date"]).replacingOccurrences(of: "-", with: ""))-\(number)"
        default: return "\(ruc)-\(field(object["document_type"]))-\(field(object["series"]))-\(number)"
        }
    }

    /// Both texts are XML and equal under C14N.
    static func sameCanonical(_ left: String, _ right: String) -> Bool {
        guard let left = canonical(left), let right = canonical(right) else { return false }
        return left == right
    }

    /// Inclusive C14N 1.0 without comments, from libxml2; nil if the text is not XML.
    static func canonical(_ xml: String) -> String? {
        var bytes = Array(xml.utf8)
        bytes.append(0)
        guard let out = bytes.withUnsafeBufferPointer({ buffer in
            buffer.withMemoryRebound(to: CChar.self) { battery_c14n($0.baseAddress, buffer.count - 1) }
        }) else { return nil }
        defer { battery_free(out) }
        return String(cString: out)
    }

    static func platform() -> String {
        #if os(macOS)
        let system = "macOS"
        #elseif os(iOS)
        let system = "iOS"
        #elseif os(Linux)
        let system = "Linux"
        #else
        let system = "other"
        #endif
        #if arch(arm64)
        let arch = "arm64"
        #elseif arch(x86_64)
        let arch = "x86_64"
        #else
        let arch = "other"
        #endif
        #if targetEnvironment(simulator)
        let where_ = " simulator"
        #else
        let where_ = ""
        #endif
        return "\(system)\(where_) \(ProcessInfo.processInfo.operatingSystemVersionString) \(arch)"
    }
}
