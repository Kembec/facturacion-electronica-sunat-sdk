# frozen_string_literal: true

module Kembec
  module SunatSdk
    # validity: the voucher's state in the Consulta Integrada (:validity), SUNAT refusing the query (:query_error),
    # or :no_verdict.
    class ValidityResult < Result
      attr_reader :outcome
      attr_reader :state_code
      # :not_reported (0; it does not prove the voucher never arrived), :accepted (1), :voided (2),
      # :printed_authorized (3), :printed_not_authorized (4) or :unknown.
      attr_reader :state
      attr_reader :taxpayer_state_code
      attr_reader :domicile_condition_code
      # Where the data sent differs from SUNAT's record.
      attr_reader :observations
      attr_reader :message
      attr_reader :query_error
      attr_reader :no_verdict

      def initialize(value, raw_json)
        super(raw_json)
        @outcome = Fields.outcome(value, :validity, :query_error, :no_verdict)
        @state = :unknown
        @observations = [].freeze
        case @outcome
        when :validity then read_validity(value)
        when :query_error then @query_error = QueryError.new(value)
        else @no_verdict = NoVerdict.new(value)
        end
        freeze
      end

      private

      def read_validity(value)
        @state_code = Fields.optional_text(value, "state_code")
        @state = Fields.validity_state(value)
        @taxpayer_state_code = Fields.optional_text(value, "taxpayer_state_code")
        @domicile_condition_code = Fields.optional_text(value, "domicile_condition_code")
        @observations = Fields.texts(value, "observations")
        @message = Fields.optional_text(value, "message")
      end
    end

    # SUNAT refused the Consulta Integrada query (about the query, not the voucher).
    class QueryError
      attr_reader :http_status
      # "99" is a malformed query.
      attr_reader :error_code
      attr_reader :message

      def initialize(value)
        @http_status = Fields.int(value, "http_status")
        @error_code = Fields.optional_text(value, "error_code")
        @message = Fields.text(value, "message")
        freeze
      end
    end

    # gre_send: SUNAT received the guide and gave a ticket (:ticket), a GRE API error (:api_error), or :no_verdict
    # of the token (nothing was sent).
    class GreSendResult < Result
      attr_reader :outcome
      # The only handle on the outcome: store it before anything else.
      attr_reader :ticket
      attr_reader :received_at
      attr_reader :zip_name
      # SHA-256 of the zip, lowercase hex.
      attr_reader :hash_zip
      attr_reader :api_error
      attr_reader :no_verdict

      def initialize(value, raw_json)
        super(raw_json)
        @outcome = Fields.outcome(value, :ticket, :api_error, :no_verdict)
        case @outcome
        when :ticket
          @ticket = Fields.text(value, "ticket")
          @received_at = Fields.optional_text(value, "received_at")
          @zip_name = Fields.optional_text(value, "zip_name")
          @hash_zip = Fields.optional_text(value, "hash_zip")
        when :api_error then @api_error = GreApiError.new(value)
        else @no_verdict = NoVerdict.new(value)
        end
        freeze
      end
    end

    # gre_ticket: the state of a sent guide (:ticket_status), a GRE API error, or :no_verdict.
    class GreTicketResult < Result
      attr_reader :outcome
      attr_reader :ticket
      attr_reader :status_code
      # :processed (0, with its cdr), :in_process (98: ask again with the same ticket), :processed_with_errors (99,
      # with its error) or :unknown.
      attr_reader :state
      attr_reader :error
      attr_reader :cdr
      attr_reader :cdr_zip
      attr_reader :api_error
      attr_reader :no_verdict

      def initialize(value, raw_json)
        super(raw_json)
        @outcome = Fields.outcome(value, :ticket_status, :api_error, :no_verdict)
        @ticket = Fields.text(value, "ticket")
        @state = :unknown
        @cdr_generated = false
        case @outcome
        when :ticket_status then read_status(value)
        when :api_error then @api_error = GreApiError.new(value)
        else @no_verdict = NoVerdict.new(value)
        end
        freeze
      end

      # Whether SUNAT generated the guide's CDR.
      def cdr_generated?
        @cdr_generated
      end

      private

      def read_status(value)
        @status_code = Fields.text(value, "status_code")
        @state = Fields.ticket_state(value)
        @cdr_generated = Fields.bool(value, "cdr_generated")
        error = Fields.optional_object(value, "error")
        @error = error && GreTicketError.new(error)
        cdr = Fields.optional_object(value, "cdr")
        @cdr = cdr && Cdr.new(cdr)
        @cdr_zip = Fields.optional_base64(value, "cdr_zip_base64")
      end
    end

    # The error of a guide processed with errors (numError, desError).
    class GreTicketError
      attr_reader :code
      attr_reader :description
      attr_reader :response_class

      def initialize(value)
        @code = Fields.optional_text(value, "code")
        @description = Fields.text(value, "description")
        @response_class = Fields.response_class(value)
        freeze
      end
    end

    # An error body of the GRE REST API (annex I of SUNAT's GRE manual).
    class GreApiError
      # :token, :send or :ticket.
      attr_reader :stage
      attr_reader :http_status
      # Annex I's status name ("unauthorized", "unprocessable_entity"...).
      attr_reader :general_error
      attr_reader :code
      attr_reader :message
      # The validation errors (a 422 lists them), each with code and description.
      attr_reader :errors

      Item = Struct.new(:code, :description)

      def initialize(value)
        @stage = Fields.stage(value)
        @http_status = Fields.int(value, "http_status")
        @general_error = Fields.optional_text(value, "general_error")
        @code = Fields.optional_text(value, "code")
        @message = Fields.text(value, "message")
        list = value["errors"].is_a?(Array) ? value["errors"] : []
        @errors = list.map do |item|
          raise Native::ShapeError, "an item of `errors` is not an object" unless item.is_a?(Hash)

          Item.new(Fields.optional_text(item, "code"), Fields.text(item, "description")).freeze
        end.freeze
        freeze
      end
    end
  end
end
