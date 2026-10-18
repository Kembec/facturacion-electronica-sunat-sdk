# frozen_string_literal: true

module Kembec
  module SunatSdk
    # A code SUNAT answered, with the input fields that may have caused it (the `diagnosis` of docs/cli.md).
    class Diagnosis
      # SUNAT's code ("2800", "4287"...).
      attr_reader :code
      # What SUNAT's message names (node and value), or nil when it names none.
      attr_reader :detail
      # The candidate fields, DiagnosisField each; empty for a code the rules do not list, and for summaries, void
      # communications and tickets. With a detail, only those on its node, unless none is.
      attr_reader :fields

      def initialize(value)
        @code = Fields.text(value, "code")
        detail = Fields.optional_object(value, "detail")
        @detail = detail && DiagnosisDetail.new(detail)
        @fields = Fields.objects(value, "fields").map { |field| DiagnosisField.new(field) }.freeze
        freeze
      end

      # The `diagnosis` of an outcome: one per code in the answer (the fault's, a CDR's non-zero response_code, each
      # note that starts with a code); empty when the output has none.
      def self.list_from(value)
        Fields.objects(value, "diagnosis").map { |item| new(item) }.freeze
      end
    end

    # What SUNAT's message names as (nodo: "<node>" valor: "<value>"), e.g. "cbc:ID/schemeID" = "1". The form is
    # observed in beta answers; SUNAT does not document it.
    class DiagnosisDetail
      # The last element and, if any, its attribute, as SUNAT writes it.
      attr_reader :node
      # The value SUNAT read in that node.
      attr_reader :value

      def initialize(value)
        @node = Fields.text(value, "node")
        @value = Fields.text(value, "value")
        freeze
      end
    end

    # One candidate field for a code, from SUNAT's validation rules of 26.08.2026 (docs/cli.md#diagnosis).
    class DiagnosisField
      # A field of docs/input-model.md ("customer.document_type"), "sdk" when the SDK writes the node, or nil when the
      # input model has no such field.
      attr_reader :input_path
      # The node the rule checks; nil for rules on the file, the signature or the service.
      attr_reader :xpath
      # "fuente" when the node is the rule's own and one field writes it, "inferido" when the rule computes over
      # several nodes.
      attr_reader :certainty

      def initialize(value)
        @input_path = Fields.optional_text(value, "input_path")
        @xpath = Fields.optional_text(value, "xpath")
        @certainty = Fields.text(value, "certainty")
        freeze
      end
    end
  end
end
