import Foundation
import KembecSunatSDK

/// `diagnosis` on the four SOAP results. The answers are the core's own output (bindings/swift/Battery/diagnosis, the
/// files the PHP and wasm bindings check): send-beta-2800 is the beta's real answer to a boleta (17/10/2026).
struct DiagnosisChecks {
    let checker: Checker
    let repo: Repository

    func run() throws {
        let decoder = JSONDecoder()
        let faulted = try decoder.decode(SendResult.self, from: Data(repo.text("bindings/swift/Battery/diagnosis/send-beta-2800.json").utf8))
        if case let .fault(fault) = faulted.outcome {
            checker.check(fault.code == "2800" && fault.responseClass == .rejected, "diagnosis: the 2800 is a rejected fault")
        } else {
            checker.check(false, "diagnosis: the 2800 is not a fault")
        }
        checker.check(faulted.diagnosis.count == 1 && faulted.diagnosis[0].code == "2800", "diagnosis: \(faulted.diagnosis)")
        let diagnosis = faulted.diagnosis.first
        checker.check(diagnosis?.detail?.node == "cbc:ID/schemeID" && diagnosis?.detail?.value == "1", "diagnosis: detail \(String(describing: diagnosis?.detail))")
        checker.check(diagnosis?.fields.count == 1 && diagnosis?.fields[0].inputPath == "customer.document_type", "diagnosis: fields \(String(describing: diagnosis?.fields))")
        checker.check(diagnosis?.fields.first?.certainty == .source, "diagnosis: certainty")
        checker.check(diagnosis?.fields.first?.xpath?.hasSuffix("cbc:ID@schemeID") == true, "diagnosis: xpath")
        let again = try decoder.decode(SendResult.self, from: JSONEncoder().encode(faulted))
        checker.check(again.diagnosis == faulted.diagnosis, "diagnosis: Codable round trip")

        let observed = try decoder.decode(StatusResult.self, from: Data(repo.text("bindings/swift/Battery/diagnosis/status-observed-4287.json").utf8))
        checker.check(observed.diagnosis.map(\.code) == ["4287", "4001"], "diagnosis: observed notes \(observed.diagnosis)")
        checker.check(observed.diagnosis.first?.detail == nil && observed.diagnosis.first?.fields.first?.inputPath == "lines[].unit_price" &&
                      observed.diagnosis.first?.fields.first?.certainty == .inferred, "diagnosis: 4287")
        checker.check(observed.diagnosis.last?.fields.isEmpty == true, "diagnosis: 4001 is not in the 2026 rules")

        let withoutDiagnosis = [
            "send": try decoder.decode(SendResult.self, from: Data(#"{"outcome":"fault","code":"1033","message":"x","class":"already_registered"}"#.utf8)).diagnosis,
            "summary": try decoder.decode(SummaryResult.self, from: Data(#"{"outcome":"ticket","ticket":"1"}"#.utf8)).diagnosis,
            "ticket": try decoder.decode(TicketResult.self, from: Data(#"{"outcome":"no_verdict","reason":"timeout","message":"x","ticket":"1"}"#.utf8)).diagnosis,
            "status": try decoder.decode(StatusResult.self, from: Data(#"{"outcome":"no_verdict","reason":"timeout","message":"x"}"#.utf8)).diagnosis,
        ]
        for (name, list) in withoutDiagnosis {
            checker.check(list.isEmpty, "diagnosis: \(name) without `diagnosis` is \(list)")
        }
    }
}
