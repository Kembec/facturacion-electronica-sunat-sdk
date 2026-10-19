# frozen_string_literal: true

require_relative "sunat_sdk/version"
require_relative "sunat_sdk/errors"
require_relative "sunat_sdk/credentials"
require_relative "sunat_sdk/native"
require_relative "sunat_sdk/fields"
require_relative "sunat_sdk/diagnosis"
require_relative "sunat_sdk/results"
require_relative "sunat_sdk/soap_results"
require_relative "sunat_sdk/rest_results"
require_relative "sunat_sdk/client"

module Kembec
  # SUNAT (Peru) electronic invoicing: the SDK's Rust core through its C ABI. Start with Client.
  module SunatSdk
    # The version of the native library loaded ("1.0.0-rc.5"), equal to SDK_VERSION. Loads the library: raises
    # NativeLibraryError if it cannot.
    def self.native_version
      Native.version
    end
  end
end
