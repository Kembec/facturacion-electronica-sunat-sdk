import Foundation
import SunatSDKBattery
import XCTest
#if canImport(SunatSDKObjCBattery)
import SunatSDKObjCBattery
#endif

/// The two batteries as XCTest, for where an executable does not run: the iOS simulator. The repository is this file's
/// checkout (the simulator reads the host's disk); the batteries are the ones swift-battery and objc-battery run.
final class BatteryTests: XCTestCase {
    private let repository = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent() // BatteryTests
        .deletingLastPathComponent() // Tests
        .deletingLastPathComponent() // swift
        .deletingLastPathComponent() // bindings
        .deletingLastPathComponent()

    func testSwiftBattery() {
        let report = SwiftBattery.run(repository: repository)
        print(report.summary)
        XCTAssertEqual(report.failures, [], report.summary)
        XCTAssertEqual(report.vectors, SwiftBattery.expectedFixtures)
    }

    #if canImport(SunatSDKObjCBattery)
    func testObjectiveCBattery() {
        let report = KSSObjCBattery.run(withRepository: repository)
        print(report.summary)
        XCTAssertEqual(report.failures, [], report.summary)
        XCTAssertEqual(report.vectors, KSSObjCBattery.expectedFixtures)
    }
    #endif
}
