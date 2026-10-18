# frozen_string_literal: true

module Kembec
  module SunatSdk
    # The SDK's version (Cargo.toml), as the native library reports it.
    SDK_VERSION = "0.1.0-beta.7"

    # The gem's version: RubyGems spells a prerelease with dots (0.1.0.beta.7).
    VERSION = SDK_VERSION.tr("-", ".")

    # The C ABI this gem speaks (docs/c-abi.md); a library with another one is refused at load.
    ABI_VERSION = 1
  end
end
