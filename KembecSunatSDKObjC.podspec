# The Objective-C package for CocoaPods, served from Git: `pod 'KembecSunatSDKObjC', :git => '…', :tag => 'v…'`. The
# same sources and the same SunatSDKFFI.xcframework as the KembecSunatSDKObjC product of Package.swift (bindings/objc).
Pod::Spec.new do |s|
  s.name = "KembecSunatSDKObjC"
  s.version = "0.1.0-beta.4"
  s.summary = "SUNAT (Peru) electronic invoicing: build, sign, send and query vouchers, summaries and despatch advices"
  s.description = "Factura, boleta, notas, resumen diario, comunicación de baja y guía de remisión remitente: arma el " \
                  "XML UBL 2.1, lo firma, lo envía a la SUNAT y lee la CDR. The SDK's Rust core through its C ABI, " \
                  "linked as a static library."
  s.homepage = "https://fe.kembec.com"
  s.license = { :type => "MIT", :file => "LICENSE" }
  s.author = "Manuel Benancio"
  s.source = { :git => "https://github.com/Kembec/facturacion-electronica-sunat-sdk.git", :tag => "v#{s.version}" }
  s.documentation_url = "https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/bindings/objc/README.md"

  s.ios.deployment_target = "13.0"
  s.osx.deployment_target = "10.15"
  s.requires_arc = true

  s.source_files = "bindings/objc/Sources/KembecSunatSDKObjC/**/*.{h,m}"
  s.public_header_files = "bindings/objc/Sources/KembecSunatSDKObjC/include/KembecSunatSDKObjC/*.h"
  s.header_dir = "KembecSunatSDKObjC"
  s.module_name = "KembecSunatSDKObjC"
  s.frameworks = "Foundation"
  # Downloaded by prepare_command and checked against Package.swift's checksum: never committed.
  s.vendored_frameworks = "bindings/swift/artifacts/SunatSDKFFI.xcframework"
  s.prepare_command = "bindings/objc/scripts/fetch-xcframework.sh"
end
