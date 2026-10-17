# frozen_string_literal: true

module Kembec
  module SunatSdk
    # A command that did not run: the SDK's own error (the CLI's {"error"} with exit code 1, such as an invalid
    # document, a missing certificate or a send without an answer) or a response that is not the contract's.
    # An answer from SUNAT, a rejection included, is a result, never this error.
    class Error < StandardError
      # The command of docs/cli.md that failed ("build", "send"...); nil if none ran.
      attr_reader :command

      def initialize(message, command: nil)
        super(message)
        @command = command
      end
    end

    # The native library was not found, could not be loaded into this process, or speaks another ABI. Nothing ran.
    class NativeLibraryError < Error
    end
  end
end
