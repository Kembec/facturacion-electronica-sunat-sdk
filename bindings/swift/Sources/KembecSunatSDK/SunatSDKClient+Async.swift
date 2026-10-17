import Dispatch
import Foundation

// The network commands for Swift concurrency. A command blocks its thread for up to the timeout; on the cooperative
// pool that would starve every other task, so each call runs on a global dispatch queue and the task awaits it.
extension SunatSDKClient {
    public func send(name: String, signedXML: String, credentials: SOLCredentials, endpoint: URL) async throws -> SendResult {
        try await offload { try self.send(name: name, signedXML: signedXML, credentials: credentials, endpoint: endpoint) }
    }

    public func summary(name: String, signedXML: String, credentials: SOLCredentials, endpoint: URL) async throws -> SummaryResult {
        try await offload { try self.summary(name: name, signedXML: signedXML, credentials: credentials, endpoint: endpoint) }
    }

    public func ticket(_ ticket: String, credentials: SOLCredentials, endpoint: URL) async throws -> TicketResult {
        try await offload { try self.ticket(ticket, credentials: credentials, endpoint: endpoint) }
    }

    public func status(ruc: String, documentType: String, series: String, number: Int64, credentials: SOLCredentials,
                       endpoint: URL) async throws -> StatusResult {
        try await offload {
            try self.status(ruc: ruc, documentType: documentType, series: series, number: number, credentials: credentials, endpoint: endpoint)
        }
    }

    public func validity(ruc: String, documentType: String, series: String, number: Int64, issueDate: DateComponents,
                         total: Decimal, credentials: APICredentials, requesterRUC: String? = nil, tokenEndpoint: URL? = nil,
                         endpoint: URL? = nil) async throws -> ValidityResult {
        try await offload {
            try self.validity(ruc: ruc, documentType: documentType, series: series, number: number, issueDate: issueDate, total: total,
                              credentials: credentials, requesterRUC: requesterRUC, tokenEndpoint: tokenEndpoint, endpoint: endpoint)
        }
    }

    public func greSend(name: String, signedXML: String, api: APICredentials, sol: SOLCredentials, tokenEndpoint: URL,
                        endpoint: URL) async throws -> GRESendResult {
        try await offload {
            try self.greSend(name: name, signedXML: signedXML, api: api, sol: sol, tokenEndpoint: tokenEndpoint, endpoint: endpoint)
        }
    }

    public func greTicket(_ ticket: String, api: APICredentials, sol: SOLCredentials, tokenEndpoint: URL,
                          endpoint: URL) async throws -> GRETicketResult {
        try await offload { try self.greTicket(ticket, api: api, sol: sol, tokenEndpoint: tokenEndpoint, endpoint: endpoint) }
    }

    private func offload<T: Sendable>(_ work: @escaping @Sendable () throws -> T) async throws -> T {
        try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global(qos: .userInitiated).async {
                continuation.resume(with: Result { try work() })
            }
        }
    }
}
