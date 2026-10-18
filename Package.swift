// swift-tools-version:5.9
// The SDK for Swift and Objective-C over its C ABI (docs/c-abi.md). SwiftPM reads a package only from the root of its
// repository, so this manifest lives here and the sources under bindings/swift and bindings/objc.
import PackageDescription

// The C ABI is the only native piece. On Apple platforms it is an .xcframework of static libraries (macOS arm64 and
// x86_64, iOS, iOS simulator) built by .github/workflows/swift.yml; a release fills `release` with the URL and checksum
// of its zip in the tagged commit. While it is nil, the package takes the xcframework from bindings/swift/artifacts,
// where the CI artifact is unzipped (bindings/swift/README.md).
let release: (url: String, checksum: String)? = (url: "https://github.com/Kembec/facturacion-electronica-sunat-sdk/releases/download/v1.0.0-rc.3/SunatSDKFFI.xcframework.zip", checksum: "828d2b43ff5478456fff0869e0e62d78c3b0dee69e25864b942ab822f860df6a")

// libxml2 comes with the SDK on Apple platforms (headers in usr/include/libxml, libxml2.tbd): no pkg-config there, or a
// Homebrew libxml2 found by it would be linked into the macOS build and break the iOS one.
#if os(Linux)
let libxml2PkgConfig: String? = "libxml-2.0"
#else
let libxml2PkgConfig: String? = nil
#endif

var products: [Product] = [
    .library(name: "KembecSunatSDK", targets: ["KembecSunatSDK"]),
]

var targets: [Target] = [
    .target(
        name: "KembecSunatSDK",
        dependencies: ["SunatSDKFFI"],
        path: "bindings/swift/Sources/KembecSunatSDK"
    ),
    // The battery: the 38 vectors and the contract's edges through each wrapper. Not a product: no consumer builds it.
    .systemLibrary(
        name: "CLibXML2",
        path: "bindings/swift/Battery/CLibXML2",
        pkgConfig: libxml2PkgConfig,
        providers: [.apt(["libxml2-dev"])]
    ),
    .target(
        name: "CBatterySupport",
        dependencies: ["CLibXML2"],
        path: "bindings/swift/Battery/CBatterySupport"
    ),
    .target(
        name: "SunatSDKBattery",
        dependencies: ["KembecSunatSDK", "CBatterySupport"],
        path: "bindings/swift/Battery/SunatSDKBattery"
    ),
    .executableTarget(
        name: "swift-battery",
        dependencies: ["SunatSDKBattery"],
        path: "bindings/swift/Battery/swift-battery"
    ),
]

var batteryTestDependencies: [Target.Dependency] = ["SunatSDKBattery"]

#if os(Linux)
// No .xcframework on Linux: the shared library of bindings/ffi, installed where the linker finds it.
targets.append(.systemLibrary(name: "SunatSDKFFI", path: "bindings/swift/Sources/SunatSDKFFI"))
#else
// KEMBEC_SUNAT_LOCAL_XCFRAMEWORK=1 (set by swift.yml) takes the local xcframework even with `release` pinned: the CI
// tests what its own run built, never the URL, which does not exist until the tag's release attaches it. A consumer
// never sets it.
let useLocalXCFramework = Context.environment["KEMBEC_SUNAT_LOCAL_XCFRAMEWORK"] == "1"
if let release, !useLocalXCFramework {
    targets.append(.binaryTarget(name: "SunatSDKFFI", url: release.url, checksum: release.checksum))
} else {
    targets.append(.binaryTarget(name: "SunatSDKFFI", path: "bindings/swift/artifacts/SunatSDKFFI.xcframework"))
}
// Objective-C needs the Apple runtime and Foundation: Apple platforms only.
products.append(.library(name: "KembecSunatSDKObjC", targets: ["KembecSunatSDKObjC"]))
targets += [
    .target(
        name: "KembecSunatSDKObjC",
        dependencies: ["SunatSDKFFI"],
        path: "bindings/objc/Sources/KembecSunatSDKObjC",
        publicHeadersPath: "include"
    ),
    .target(
        name: "SunatSDKObjCBattery",
        dependencies: ["KembecSunatSDKObjC", "CBatterySupport"],
        path: "bindings/objc/Battery/SunatSDKObjCBattery",
        publicHeadersPath: "include"
    ),
    .executableTarget(
        name: "objc-battery",
        dependencies: ["SunatSDKObjCBattery"],
        path: "bindings/objc/Battery/objc-battery"
    ),
]
batteryTestDependencies.append("SunatSDKObjCBattery")
#endif

// The same two batteries as an XCTest, for the iOS simulator, where an executable does not run.
targets.append(.testTarget(
    name: "BatteryTests",
    dependencies: batteryTestDependencies,
    path: "bindings/swift/Tests/BatteryTests"
))

let package = Package(
    name: "KembecSunatSDK",
    platforms: [.macOS(.v10_15), .iOS(.v13)],
    products: products,
    targets: targets
)
