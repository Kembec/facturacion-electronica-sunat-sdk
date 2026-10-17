import CBatterySupport
import Dispatch
import Foundation
import KembecSunatSDK

/// Counts checks and collects failures; safe from several threads.
final class Checker: @unchecked Sendable {
    private let lock = NSLock()
    private var checks = 0
    private var failures: [String] = []

    func check(_ ok: Bool, _ what: @autoclosure () -> String) {
        lock.lock()
        defer { lock.unlock() }
        checks += 1
        if !ok { failures.append(what()) }
    }

    /// Runs `body`; anything it throws is a failed check named `what`.
    func run(_ what: String, _ body: () throws -> Void) {
        do {
            try body()
        } catch {
            check(false, "\(what): \(error)")
        }
    }

    /// Runs `body`, which must throw a ``SunatSDKError``; returns it, or nil after a failed check.
    func expectError(_ what: String, _ body: () throws -> Void) -> SunatSDKError? {
        do {
            try body()
            check(false, "\(what): no error")
        } catch let error as SunatSDKError {
            check(true, what)
            return error
        } catch {
            check(false, "\(what): \(type(of: error)) \(error)")
        }
        return nil
    }

    func totals() -> (Int, [String]) {
        lock.lock()
        defer { lock.unlock() }
        return (checks, failures)
    }
}

/// A value behind a lock.
final class Locked<Value>: @unchecked Sendable {
    private let lock = NSLock()
    private var stored: Value

    init(_ value: Value) {
        stored = value
    }

    var value: Value {
        lock.lock()
        defer { lock.unlock() }
        return stored
    }

    func update(_ change: (inout Value) -> Void) {
        lock.lock()
        defer { lock.unlock() }
        change(&stored)
    }
}

/// The checked-out repository: fixtures, vectors, the test certificate and the recorded CDR.
/// Holds the root as a path: `URL` is not `Sendable` in swift-corelibs-foundation 5.9.
struct Repository: Sendable {
    let rootPath: String

    init(root: URL) {
        rootPath = root.path
    }

    func file(_ path: String) -> URL {
        URL(fileURLWithPath: rootPath, isDirectory: true).appendingPathComponent(path)
    }

    func text(_ path: String) throws -> String {
        try String(contentsOf: file(path), encoding: .utf8)
    }

    func vector(_ name: String, _ kind: String) throws -> String {
        try text("tests/vectors/\(name).\(kind).xml")
    }

    /// The fixture names, sorted, without `.json`.
    func fixtures() -> [String] {
        let names = (try? FileManager.default.contentsOfDirectory(atPath: file("fixtures").path)) ?? []
        return names.filter { $0.hasSuffix(".json") }.map { String($0.dropLast(".json".count)) }.sorted()
    }
}

/// The network commands against a listener that never answers: each query waits out its 1 s and has no verdict.
struct SilentChecks {
    let checker: Checker

    func run() throws {
        let port = battery_silent_listener_start()
        guard port > 0 else {
            checker.check(false, "no_verdict: no local listener")
            return
        }
        defer { battery_silent_listener_stop() }
        let silent = "https://127.0.0.1:\(port)"
        let client = try SunatSDKClient(httpTimeout: 1)
        let sol = SOLCredentials(user: "20123456789MODDATOS", password: "moddatos")
        let api = APICredentials(clientID: "client-id", clientSecret: "client-secret")

        let status = try client.status(ruc: "20123456789", documentType: "01", series: "F001", number: 1, credentials: sol,
                                       endpoint: URL(string: silent + "/billConsultService")!)
        if case let .noVerdict(noVerdict) = status.outcome {
            checker.check(noVerdict.reason == .timeout && noVerdict.httpStatus == nil, "status without an answer: \(status.rawJSON)")
        } else {
            checker.check(false, "status without an answer: \(status.rawJSON)")
        }
        let again = try JSONDecoder().decode(StatusResult.self, from: JSONEncoder().encode(status))
        checker.check(again.outcome == status.outcome && again.rawJSON.isEmpty, "status: Codable round trip")

        let validity = try client.validity(ruc: "20123456789", documentType: "01", series: "F001", number: 1,
                                           issueDate: DateComponents(year: 2026, month: 10, day: 1), total: Decimal(string: "118.00")!,
                                           credentials: api, tokenEndpoint: URL(string: silent + "/token")!, endpoint: URL(string: silent + "/query")!)
        if case let .noVerdict(noVerdict) = validity.outcome {
            checker.check(noVerdict.stage == .token, "validity without a token: \(validity.rawJSON)")
        } else {
            checker.check(false, "validity without a token: \(validity.rawJSON)")
        }

        let gre = try client.greTicket("1700000000000", api: api, sol: sol, tokenEndpoint: URL(string: silent + "/token")!,
                                       endpoint: URL(string: silent + "/v1")!)
        if case let .noVerdict(noVerdict) = gre.outcome {
            checker.check(noVerdict.stage == .token && gre.ticket == "1700000000000", "gre-ticket without a token: \(gre.rawJSON)")
        } else {
            checker.check(false, "gre-ticket without a token: \(gre.rawJSON)")
        }
        try asyncCalls(client, sol, silent)
    }

    /// Four awaited queries at once: the async overloads run off the cooperative pool, so they wait side by side, about
    /// 1 s in all; one after another, or a starved pool, would take 4 s or more.
    private func asyncCalls(_ client: SunatSDKClient, _ sol: SOLCredentials, _ silent: String) throws {
        let endpoint = URL(string: silent + "/billConsultService")!
        let outcome = Locked<(Int, TimeInterval)?>(nil)
        let done = DispatchSemaphore(value: 0)
        Task {
            let start = Date()
            let verdicts = await withTaskGroup(of: Bool.self) { group -> Int in
                for number in 1...4 {
                    group.addTask {
                        let result = try? await client.status(ruc: "20123456789", documentType: "01", series: "F001", number: Int64(number),
                                                              credentials: sol, endpoint: endpoint)
                        if case .noVerdict? = result?.outcome { return true }
                        return false
                    }
                }
                return await group.reduce(0) { $0 + ($1 ? 1 : 0) }
            }
            outcome.update { $0 = (verdicts, Date().timeIntervalSince(start)) }
            done.signal()
        }
        done.wait()
        let (verdicts, elapsed) = outcome.value ?? (0, 0)
        checker.check(verdicts == 4, "async: \(verdicts) of 4 queries without a verdict")
        checker.check(elapsed < 3, "async: 4 queries of 1 s took \(String(format: "%.1f", elapsed)) s")
    }
}
