# frozen_string_literal: true

require_relative "lib/kembec/sunat_sdk/version"

# One gem per platform, each with its native library inside (scripts/build-gems.sh stages it and sets the platform).
# There is no gem without a platform: without its library the gem cannot run.
platform = ENV.fetch("SUNAT_SDK_GEM_PLATFORM") do
  raise "set SUNAT_SDK_GEM_PLATFORM (x86_64-linux-gnu, arm64-darwin...): build with scripts/build-gems.sh"
end

Gem::Specification.new do |spec|
  spec.name = "kembec-sunat_sdk"
  spec.version = Kembec::SunatSdk::VERSION
  spec.platform = Gem::Platform.new(platform)
  spec.authors = ["Manuel Benancio"]
  spec.summary = "SUNAT (Peru) electronic invoicing: build, sign, send and query vouchers, summaries and despatch advices"
  spec.description = "Factura, boleta, notas, resumen diario, comunicación de baja y guía de remisión remitente: " \
                     "arma el XML UBL 2.1, lo firma, lo envía a la SUNAT y lee la CDR. The SDK's Rust core through " \
                     "its C ABI, with the native library inside."
  spec.homepage = "https://fe.kembec.com"
  spec.license = "MIT"
  spec.metadata = {
    "source_code_uri" => "https://github.com/Kembec/facturacion-electronica-sunat-sdk/tree/main/bindings/ruby",
    "bug_tracker_uri" => "https://github.com/Kembec/facturacion-electronica-sunat-sdk/issues",
    "documentation_uri" => "https://github.com/Kembec/facturacion-electronica-sunat-sdk/blob/main/bindings/ruby/README.md",
    "rubygems_mfa_required" => "true"
  }
  # The oldest Ruby the CI runs the battery on.
  spec.required_ruby_version = ">= 3.2"
  # RubyGems 3.3.22 tells x86_64-linux-gnu from -musl; Ruby 3.2 ships 3.4.
  spec.required_rubygems_version = ">= 3.3.22"
  spec.files = Dir["lib/**/*.rb", "lib/kembec/sunat_sdk/native/*", "README.md", "LICENSE"]
  spec.require_paths = ["lib"]
  spec.add_dependency "ffi", "~> 1.17"
end
