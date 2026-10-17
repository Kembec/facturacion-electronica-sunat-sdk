# frozen_string_literal: true

module Kembec
  module SunatSdk
    # Reads the fields of a response (docs/cli.md) and names its values as symbols. A field that is missing or of
    # another type raises Native::ShapeError, which the client reports as Error.
    module Fields
      CLASSES = %w[accepted observed exception already_registered number_voided_or_rejected rejected
                   void_already_reported].to_h { |name| [name, name.to_sym] }.freeze
      TICKET_STATES = %w[processed in_process processed_with_errors].to_h { |name| [name, name.to_sym] }.freeze
      VALIDITY_STATES = %w[not_reported accepted voided printed_authorized printed_not_authorized]
                        .to_h { |name| [name, name.to_sym] }.freeze
      STAGES = %w[token query send ticket].to_h { |name| [name, name.to_sym] }.freeze
      REASONS = %w[timeout http_status].to_h { |name| [name, name.to_sym] }.freeze

      module_function

      def text(value, name)
        field = optional_text(value, name)
        raise Native::ShapeError, "`#{name}` is missing" if field.nil?

        field
      end

      def optional_text(value, name)
        field = value[name]
        return nil if field.nil?
        raise Native::ShapeError, "`#{name}` is not a string" unless field.is_a?(String)

        field.freeze
      end

      def int(value, name)
        field = optional_int(value, name)
        raise Native::ShapeError, "`#{name}` is missing" if field.nil?

        field
      end

      def optional_int(value, name)
        field = value[name]
        return nil if field.nil?
        raise Native::ShapeError, "`#{name}` is not a whole number" unless field.is_a?(Integer)

        field
      end

      def bool(value, name)
        field = value[name]
        raise Native::ShapeError, "`#{name}` is not a boolean" unless [true, false].include?(field)

        field
      end

      def texts(value, name)
        field = value[name]
        return [].freeze if field.nil?
        raise Native::ShapeError, "`#{name}` is not a list of strings" unless field.is_a?(Array) && field.all?(String)

        field.map(&:freeze).freeze
      end

      # The objects of a list; an absent list is empty.
      def objects(value, name)
        field = value[name]
        return [].freeze if field.nil?
        raise Native::ShapeError, "`#{name}` is not a list of objects" unless field.is_a?(Array) && field.all?(Hash)

        field
      end

      def optional_object(value, name)
        field = value[name]
        return nil if field.nil?
        raise Native::ShapeError, "`#{name}` is not an object" unless field.is_a?(Hash)

        field
      end

      def optional_base64(value, name)
        encoded = optional_text(value, name)
        return nil if encoded.nil?

        encoded.unpack1("m0").freeze
      rescue ArgumentError
        raise Native::ShapeError, "`#{name}` is not base64"
      end

      # The outcome as a symbol, one of those the command gives.
      def outcome(value, *allowed)
        name = text(value, "outcome")
        found = allowed.find { |outcome| outcome.to_s == name }
        raise Native::ShapeError, "outcome `#{name}` is not one this command gives" if found.nil?

        found
      end

      def response_class(value)
        CLASSES.fetch(optional_text(value, "class"), :unknown)
      end

      def ticket_state(value)
        TICKET_STATES.fetch(optional_text(value, "state"), :unknown)
      end

      def validity_state(value)
        VALIDITY_STATES.fetch(optional_text(value, "state"), :unknown)
      end

      # nil for a SOAP exchange, which has a single call.
      def stage(value)
        name = optional_text(value, "stage")
        return nil if name.nil?

        STAGES.fetch(name) { raise Native::ShapeError, "unknown stage `#{name}`" }
      end

      def reason(value)
        name = text(value, "reason")
        REASONS.fetch(name) { raise Native::ShapeError, "unknown no_verdict reason `#{name}`" }
      end
    end
  end
end
