# frozen_string_literal: true

require "ffi"
require "json"

module Kembec
  module SunatSdk
    # The C ABI of bindings/ffi (docs/c-abi.md): one JSON request in, one JSON response out. Loaded on first use, so
    # `require` never fails on a machine where the library cannot load; the error comes with the first call.
    module Native
      LIBRARY_ENV = "KEMBEC_SUNAT_SDK_LIBRARY"
      STATUS_OK = 0
      STATUS_ERROR = 1

      # The shape a result builder raises when the response is not the contract's; reported as Error.
      class ShapeError < StandardError
      end

      @lock = Mutex.new
      @library = nil

      class << self
        # The SDK version of the loaded library ("0.1.0-beta.6").
        def version
          library.sunat_sdk_version.dup.force_encoding(Encoding::UTF_8).freeze
        end

        # Where the library is read from: $KEMBEC_SUNAT_SDK_LIBRARY, or the one this platform gem carries.
        def library_path
          override = ENV.fetch(LIBRARY_ENV, "")
          return override unless override.empty?

          File.expand_path("native/#{FFI.map_library_name('sunat_sdk_ffi')}", __dir__)
        end

        # Runs one command. input_json is the JSON text of the command's input; flags maps the CLI's flag names to
        # strings or whole numbers. Yields the parsed response of a status 0; raises Error on status 1.
        def run(command, input_json, flags = nil)
          text, status = call(command, input_json, flags)
          response = parse(command, text)
          raise Error.new(error_message(response), command: command) if status == STATUS_ERROR
          raise Error.new("the library answered status #{status}", command: command) unless status == STATUS_OK

          yield response, text
        rescue ShapeError => e
          raise Error.new("unexpected response: #{e.message}", command: command)
        end

        private

        def library
          @library || @lock.synchronize { @library ||= load_library }
        end

        def load_library
          path = library_path
          bindings = Module.new
          bindings.extend(FFI::Library)
          # 32-bit Windows: the library exports __stdcall (extern "system"); a cdecl call would corrupt the stack.
          bindings.ffi_convention(:stdcall) if FFI::Platform.windows? && FFI::Platform::ADDRESS_SIZE == 32
          bindings.ffi_lib(path)
          # blocking: the call releases the GVL, so a send that waits on SUNAT does not stop the other Ruby threads.
          bindings.attach_function(:sunat_sdk_call, [:pointer, :pointer], :int32, blocking: true)
          bindings.attach_function(:sunat_sdk_free, [:pointer], :void)
          bindings.attach_function(:sunat_sdk_response_len, [:pointer], :size_t)
          bindings.attach_function(:sunat_sdk_version, [], :string)
          bindings.attach_function(:sunat_sdk_abi_version, [], :uint32)
          check_abi(bindings.sunat_sdk_abi_version)
          bindings
        rescue LoadError => e
          raise NativeLibraryError, not_found(path, e)
        end

        def check_abi(abi)
          return if abi == ABI_VERSION

          raise NativeLibraryError,
                "the native library speaks ABI #{abi} and this gem ABI #{ABI_VERSION}: " \
                "the gem and its native library come from different versions"
        end

        def not_found(path, cause)
          "the native library #{path} could not be loaded into this process " \
            "(#{RUBY_PLATFORM}, #{FFI::Platform::ADDRESS_SIZE}-bit): #{cause.message}. Platform gems exist for " \
            "x86_64-linux-gnu, aarch64-linux-gnu, x86_64-darwin, arm64-darwin, x64-mingw-ucrt and x86-mingw32; " \
            "elsewhere set #{LIBRARY_ENV} to a build of bindings/ffi."
        end

        # The request goes to native memory in pieces: only the flags carry secrets, and that small piece and the
        # native copy are zeroed after the call. The input (an XML of megabytes, at most) carries none.
        def call(command, input_json, flags)
          secret = flags ? JSON.generate(flags) : nil
          pieces = ['{"command":', JSON.generate(command), ',"input":', input_json]
          pieces.push(',"flags":', secret) if secret
          pieces << "}"
          size = pieces.sum(&:bytesize)
          request = FFI::MemoryPointer.new(:char, size + 1, true)
          slot = FFI::MemoryPointer.new(:pointer)
          begin
            offset = 0
            pieces.each do |piece|
              request.put_bytes(offset, piece)
              offset += piece.bytesize
            end
            status = library.sunat_sdk_call(request, slot)
            [take(command, slot.read_pointer), status]
          ensure
            request.clear
            secret&.bytesize&.times { |i| secret.setbyte(i, 0) }
          end
        end

        def take(command, response)
          raise Error.new("the library stored no response", command: command) if response.null?

          begin
            length = library.sunat_sdk_response_len(response)
            text = response.read_bytes(length).force_encoding(Encoding::UTF_8)
          ensure
            library.sunat_sdk_free(response)
          end
          raise Error.new("the library's response is not UTF-8", command: command) unless text.valid_encoding?

          text
        end

        def parse(command, text)
          value = JSON.parse(text)
          raise Error.new("the library's response is not a JSON object", command: command) unless value.is_a?(Hash)

          value
        rescue JSON::ParserError
          raise Error.new("the library's response is not JSON", command: command)
        end

        def error_message(response)
          message = response["error"]
          raise ShapeError, "`error` is missing" unless message.is_a?(String)

          message
        end
      end
    end
  end
end
