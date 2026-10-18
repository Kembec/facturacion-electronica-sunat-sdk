# frozen_string_literal: true

module Kembec
  module SunatSdk
    # send_bill: SUNAT's CDR (outcome :cdr) or a fault (outcome :fault).
    class SendResult < Result
      attr_reader :outcome
      # The CDR, with outcome :cdr.
      attr_reader :cdr
      # The CDR zip as SUNAT sent it, a binary String; keep it.
      attr_reader :cdr_zip
      # The fault, with outcome :fault.
      attr_reader :fault
      # The input fields behind each code of the answer, Diagnosis each; empty when the answer has none.
      attr_reader :diagnosis

      def initialize(value, raw_json)
        super(raw_json)
        @outcome = Fields.outcome(value, :cdr, :fault)
        if @outcome == :cdr
          cdr = Fields.optional_object(value, "cdr")
          raise Native::ShapeError, "`cdr` is missing" if cdr.nil?

          @cdr = Cdr.new(cdr)
          @cdr_zip = Fields.optional_base64(value, "cdr_zip_base64")
        else
          @fault = Fault.new(value)
        end
        @diagnosis = Diagnosis.list_from(value)
        freeze
      end
    end

    # summary: SUNAT received the summary or void and gave a ticket (outcome :ticket), or a fault.
    class SummaryResult < Result
      attr_reader :outcome
      # The only handle on the outcome: store it before anything else.
      attr_reader :ticket
      attr_reader :fault
      # The input fields behind each code of the answer, Diagnosis each; empty when the answer has none.
      attr_reader :diagnosis

      def initialize(value, raw_json)
        super(raw_json)
        @outcome = Fields.outcome(value, :ticket, :fault)
        if @outcome == :ticket
          @ticket = Fields.text(value, "ticket")
        else
          @fault = Fault.new(value)
        end
        @diagnosis = Diagnosis.list_from(value)
        freeze
      end
    end

    # ticket: the state of a summary's or a void's ticket (:ticket_status), a fault, or :no_verdict.
    class TicketResult < Result
      attr_reader :outcome
      attr_reader :ticket
      attr_reader :status_code
      # :processed (0), :in_process (98: ask again with the same ticket, never send again),
      # :processed_with_errors (99) or :unknown.
      attr_reader :state
      attr_reader :cdr
      attr_reader :cdr_zip
      attr_reader :fault
      attr_reader :no_verdict
      # The input fields behind each code of the answer, Diagnosis each; empty when the answer has none.
      attr_reader :diagnosis

      def initialize(value, raw_json)
        super(raw_json)
        @outcome = Fields.outcome(value, :ticket_status, :fault, :no_verdict)
        @ticket = Fields.optional_text(value, "ticket")
        @state = :unknown
        case @outcome
        when :ticket_status then read_status(value)
        when :fault then @fault = Fault.new(value)
        else @no_verdict = NoVerdict.new(value)
        end
        @diagnosis = Diagnosis.list_from(value)
        freeze
      end

      private

      def read_status(value)
        @status_code = Fields.text(value, "status_code")
        @state = Fields.ticket_state(value)
        cdr = Fields.optional_object(value, "cdr")
        @cdr = cdr && Cdr.new(cdr)
        @cdr_zip = Fields.optional_base64(value, "cdr_zip_base64")
      end
    end

    # status: getStatusCdr's answer (:status), a fault, or :no_verdict.
    class StatusResult < Result
      attr_reader :outcome
      attr_reader :status_code
      attr_reader :status_message
      # The CDR, when SUNAT attaches it.
      attr_reader :cdr
      attr_reader :cdr_zip
      attr_reader :fault
      attr_reader :no_verdict
      # The input fields behind each code of the answer, Diagnosis each; empty when the answer has none.
      attr_reader :diagnosis

      def initialize(value, raw_json)
        super(raw_json)
        @outcome = Fields.outcome(value, :status, :fault, :no_verdict)
        case @outcome
        when :status then read_status(value)
        when :fault then @fault = Fault.new(value)
        else @no_verdict = NoVerdict.new(value)
        end
        @diagnosis = Diagnosis.list_from(value)
        freeze
      end

      private

      def read_status(value)
        @status_code = Fields.text(value, "status_code")
        @status_message = Fields.optional_text(value, "status_message")
        cdr = Fields.optional_object(value, "cdr")
        @cdr = cdr && Cdr.new(cdr)
        @cdr_zip = Fields.optional_base64(value, "cdr_zip_base64")
      end
    end
  end
end
