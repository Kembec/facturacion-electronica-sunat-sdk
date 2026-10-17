# frozen_string_literal: true

module Kembec
  module SunatSdk
    # Checks shared by the credential objects and the client.
    module Arguments
      module_function

      # A non-empty String (or a Pathname, through #to_path), as a frozen UTF-8 copy.
      def text(value, name)
        value = value.to_path if value.respond_to?(:to_path)
        raise TypeError, "#{name} must be a String, not #{value.class}" unless value.is_a?(String)
        raise ArgumentError, "#{name} must not be empty" if value.empty?

        utf8(value, name)
      end

      # The same text as UTF-8: binary strings are read as UTF-8, any other encoding is converted.
      def utf8(value, name)
        copy = if value.encoding == Encoding::UTF_8 || value.encoding == Encoding::BINARY
                 value.dup.force_encoding(Encoding::UTF_8)
               else
                 value.encode(Encoding::UTF_8)
               end
        raise ArgumentError, "#{name} is not valid UTF-8" unless copy.valid_encoding?

        copy.freeze
      rescue EncodingError => e
        raise ArgumentError, "#{name} cannot be converted to UTF-8: #{e.message}"
      end
    end

    # Paths of the PEM certificate and its unencrypted RSA private key. The library reads the files; their content
    # never passes through Ruby.
    class SigningKey
      attr_reader :certificate_path, :private_key_path

      # Both paths as String or Pathname.
      def initialize(certificate_path, private_key_path)
        @certificate_path = Arguments.text(certificate_path, "certificate_path")
        @private_key_path = Arguments.text(private_key_path, "private_key_path")
        freeze
      end

      def inspect
        "#<#{self.class.name} [redacted]>"
      end
      alias to_s inspect
    end

    # The SOL user and password for send_bill, summary, ticket, status and the GRE token. They reach the library inside
    # the request, whose bytes are zeroed after the call, and never a result, an error or #inspect.
    class SolCredentials
      # user: the RUC followed by the SOL user ("20100066603MODDATOS"); password: the SOL password.
      def initialize(user, password)
        @user = Arguments.text(user, "user")
        @password = Arguments.text(password, "password")
        freeze
      end

      def inspect
        "#<#{self.class.name} [redacted]>"
      end
      alias to_s inspect

      # The flags of the C ABI; for the client only.
      def to_flags
        { "user" => @user, "password" => @password }
      end
    end

    # Client id and secret of «Credenciales de API SUNAT» (SOL): the Consulta Integrada pair for #validity, or the GRE
    # pair for #gre_send and #gre_ticket. Not the SOL user.
    class ApiCredentials
      def initialize(client_id, client_secret)
        @client_id = Arguments.text(client_id, "client_id")
        @client_secret = Arguments.text(client_secret, "client_secret")
        freeze
      end

      def inspect
        "#<#{self.class.name} [redacted]>"
      end
      alias to_s inspect

      # The flags of the C ABI; for the client only.
      def to_flags
        { "client-id" => @client_id, "client-secret" => @client_secret }
      end
    end
  end
end
