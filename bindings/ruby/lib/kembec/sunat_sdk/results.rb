# frozen_string_literal: true

module Kembec
  module SunatSdk
    # What every result keeps: the response exactly as the library gave it.
    class Result
      # The JSON of docs/cli.md for this result, for logs and for fields newer than this gem.
      attr_reader :raw_json

      def initialize(raw_json)
        @raw_json = raw_json.freeze
      end
    end

    # build: the unsigned UBL XML and the SUNAT file name without extension.
    class BuildResult < Result
      # "20123456789-01-F001-1", or "RUC-RC|RA-YYYYMMDD-N" for summaries and voids.
      attr_reader :file_name
      # The unsigned XML, UTF-8.
      attr_reader :xml

      def initialize(value, raw_json)
        super(raw_json)
        @file_name = Fields.text(value, "file_name")
        @xml = Fields.text(value, "xml")
        freeze
      end
    end

    # sign: the signed XML and the two values of its signature.
    class SignResult < Result
      attr_reader :xml
      # The DigestValue, base64: the hash printed on the voucher's representation.
      attr_reader :digest_value
      # The SignatureValue, base64.
      attr_reader :signature_value

      def initialize(value, raw_json)
        super(raw_json)
        @xml = Fields.text(value, "xml")
        @digest_value = Fields.text(value, "digest_value")
        @signature_value = Fields.text(value, "signature_value")
        freeze
      end
    end

    # zip: the zip SUNAT expects, with the XML inside under <name>.xml.
    class ZipResult < Result
      # "20123456789-01-F001-1.zip".
      attr_reader :file_name
      # The zip's bytes, a binary String.
      attr_reader :content

      def initialize(value, raw_json)
        super(raw_json)
        @file_name = Fields.text(value, "file_name")
        @content = Fields.optional_base64(value, "zip_base64")
        raise Native::ShapeError, "`zip_base64` is missing" if @content.nil?

        freeze
      end
    end

    # A CDR (constancia de recepción): the code SUNAT answered and its class.
    class Cdr < Result
      # The voucher it answers ("F001-1").
      attr_reader :reference_id
      # SUNAT's response code ("0", "2047"...).
      attr_reader :response_code
      attr_reader :description
      # The notes, each "code - text"; with code 0, every note is an observation.
      attr_reader :notes
      # How the SDK classifies the code and the notes: :accepted, :observed, :exception, :already_registered,
      # :number_voided_or_rejected, :rejected, :void_already_reported or :unknown.
      attr_reader :response_class

      def initialize(value)
        super(JSON.generate(value))
        @reference_id = Fields.text(value, "reference_id")
        @response_code = Fields.text(value, "response_code")
        @description = Fields.text(value, "description")
        @notes = Fields.texts(value, "notes")
        @response_class = Fields.response_class(value)
        freeze
      end

      # Code 0 without observations.
      def accepted?
        response_class == :accepted
      end

      # Accepted, with or without observations: the voucher is valid.
      def valid_document?
        %i[accepted observed].include?(response_class)
      end
    end

    # A SOAP fault: SUNAT refused the call with a code.
    class Fault
      # SUNAT's code ("1033", "0111"...).
      attr_reader :code
      attr_reader :message
      # As Cdr#response_class; :already_registered is 1033.
      attr_reader :response_class

      def initialize(value)
        @code = Fields.text(value, "code")
        @message = Fields.text(value, "message")
        @response_class = Fields.response_class(value)
        freeze
      end
    end

    # No answer, or an HTTP status that says nothing about the document. It is not a rejection: ask again
    # (docs/cli.md says when to resend the same signed XML with the same number).
    class NoVerdict
      # :timeout or :http_status.
      attr_reader :reason
      # The HTTP status (401...), nil after a timeout.
      attr_reader :http_status
      attr_reader :message
      # For the REST exchanges, the call without an answer (:token means nothing was sent); nil for SOAP.
      attr_reader :stage

      def initialize(value)
        @reason = Fields.reason(value)
        @http_status = Fields.optional_int(value, "http_status")
        @message = Fields.text(value, "message")
        @stage = Fields.stage(value)
        freeze
      end
    end
  end
end
