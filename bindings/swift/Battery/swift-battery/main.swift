import Foundation
import SunatSDKBattery

// swift-battery <repository>: exit 0 only if every check passed.
let arguments = CommandLine.arguments
guard arguments.count == 2 else {
    FileHandle.standardError.write(Data("usage: swift-battery <repository>\n".utf8))
    exit(2)
}
let report = SwiftBattery.run(repository: URL(fileURLWithPath: arguments[1]))
for failure in report.failures {
    FileHandle.standardError.write(Data("FAIL \(failure)\n".utf8))
}
if report.failures.isEmpty {
    print(report.summary)
} else {
    FileHandle.standardError.write(Data("\(report.summary)\n".utf8))
    exit(1)
}
