# frozen_string_literal: true

require "date"
require "json"

module Kembec
  module SunatSdk
    # The commands of the SDK's contract (docs/cli.md) in process, through its C ABI (docs/c-abi.md): the same
    # validation, XML, signature, network calls and outcomes as the CLI and every other language. Immutable and safe to
    # share between threads. Every method blocks until its command ends; a network call releases the GVL while it waits
    # on SUNAT (up to #http_timeout), so the other threads of the process keep running.
    class Client
      # 90 seconds, the CLI's default.
      DEFAULT_HTTP_TIMEOUT = 90
      MAX_HTTP_TIMEOUT = 300

      # How long a network command waits on SUNAT, in whole seconds.
      attr_reader :http_timeout

      # http_timeout: whole seconds from 1 to 300 (an Integer, or anything numeric such as 60.0 or 1.minute); the
      # token and the call of the REST commands share it.
      def initialize(http_timeout: DEFAULT_HTTP_TIMEOUT)
        @http_timeout = whole_seconds(http_timeout)
        freeze
      end

      # Renders the unsigned UBL XML of one voucher, daily summary, void communication or despatch advice.
      # document: the JSON text of docs/input-model.md, or a Hash in that shape (amounts as strings).
      def build(document)
        Native.run("build", document_json(document)) { |value, raw| BuildResult.new(value, raw) }
      end

      # Signs the XML of #build with XML-DSig (enveloped, RSA-SHA1, as SUNAT requires); signature_id is "signatureKG"
      # when nil.
      def sign(unsigned_xml, key, signature_id: nil)
        key = require_kind(key, SigningKey, "key")
        input = object("xml" => text(unsigned_xml, "unsigned_xml"), "signature_id" => optional(signature_id, "signature_id"))
        flags = { "cert" => key.certificate_path, "key" => key.private_key_path }
        Native.run("sign", input, flags) { |value, raw| SignResult.new(value, raw) }
      end

      # Zips the XML under <name>.xml, as SUNAT expects it. name: the file name without extension.
      def zip(name, xml)
        input = object("name" => text(name, "name"), "xml" => text(xml, "xml"))
        Native.run("zip", input) { |value, raw| ZipResult.new(value, raw) }
      end

      # Reads a CDR, zip or XML bytes, and classifies its response code (the cdr command).
      def read_cdr(cdr)
        raise TypeError, "cdr must be a String of bytes, not #{cdr.class}" unless cdr.is_a?(String)

        input = object("cdr_base64" => [cdr].pack("m0"))
        Native.run("cdr", input) { |value, _raw| Cdr.new(value) }
      end

      # The contract's `send`, named after SUNAT's sendBill because Object#send is Ruby's. Zips the signed XML and
      # sends it. A fault is a result; a send without an answer raises Error, since it may have arrived: send the same
      # XML again, never another number.
      def send_bill(name, signed_xml, credentials:, endpoint:)
        input = object("name" => text(name, "name"), "xml" => text(signed_xml, "signed_xml"))
        Native.run("send", input, soap(credentials, endpoint)) { |value, raw| SendResult.new(value, raw) }
      end

      # Zips a signed daily summary (RC) or void communication (RA) and sends it with sendSummary. Store the ticket
      # before anything else. A send without an answer raises, as #send_bill.
      def summary(name, signed_xml, credentials:, endpoint:)
        input = object("name" => text(name, "name"), "xml" => text(signed_xml, "signed_xml"))
        Native.run("summary", input, soap(credentials, endpoint)) { |value, raw| SummaryResult.new(value, raw) }
      end

      # Asks getStatus for a summary's or a void's ticket. No answer is :no_verdict: ask again, never send again.
      def ticket(ticket, credentials:, endpoint:)
        input = object("ticket" => text(ticket, "ticket"))
        Native.run("ticket", input, soap(credentials, endpoint)) { |value, raw| TicketResult.new(value, raw) }
      end

      # Asks getStatusCdr for one voucher (01, 03, 07, 08), with its CDR when SUNAT attaches it. No answer is
      # :no_verdict. endpoint: the billConsultService URL, https only.
      def status(ruc:, document_type:, series:, number:, credentials:, endpoint:)
        input = object(voucher_key(ruc, document_type, series, number))
        Native.run("status", input, soap(credentials, endpoint)) { |value, raw| StatusResult.new(value, raw) }
      end

      # Asks SUNAT's Consulta Integrada whether a voucher (01, 03, 07, 08) exists and in which state: a token, then the
      # query, both inside #http_timeout. There is no test environment: this is production.
      # issue_date: a Date, a Time (its own calendar date) or "YYYY-MM-DD"; total: the PayableAmount as a BigDecimal,
      # an Integer or a String, up to 2 decimals; credentials: the Consulta Integrada ApiCredentials; requester_ruc: the
      # issuer's when nil; token_endpoint and endpoint replace the production bases (https only).
      def validity(ruc:, document_type:, series:, number:, issue_date:, total:, credentials:,
                   requester_ruc: nil, token_endpoint: nil, endpoint: nil)
        fields = voucher_key(ruc, document_type, series, number)
        fields["issue_date"] = date_text(issue_date)
        fields["total"] = amount_text(total)
        fields["requester_ruc"] = optional(requester_ruc, "requester_ruc")
        flags = require_kind(credentials, ApiCredentials, "credentials").to_flags
        flags["token-endpoint"] = optional(token_endpoint, "token_endpoint")
        flags["endpoint"] = optional(endpoint, "endpoint")
        flags["timeout"] = http_timeout
        Native.run("validity", object(fields), flags.compact) { |value, raw| ValidityResult.new(value, raw) }
      end

      # Sends a signed despatch advice (09) by SUNAT's GRE REST API: a token by password, then the zip with its hash.
      # SUNAT has no test environment for GRE and the endpoints have no default. A send without an answer raises:
      # send the same file again, never the operation with another number.
      # api: the GRE ApiCredentials; sol: the SolCredentials; token_endpoint and endpoint: https bases.
      def gre_send(name, signed_xml, api:, sol:, token_endpoint:, endpoint:)
        input = object("name" => text(name, "name"), "xml" => text(signed_xml, "signed_xml"))
        Native.run("gre-send", input, gre(api, sol, token_endpoint, endpoint)) { |value, raw| GreSendResult.new(value, raw) }
      end

      # Asks for the outcome of a sent guide by its ticket (a token, then the query). :in_process (98): ask again with
      # the same ticket.
      def gre_ticket(ticket, api:, sol:, token_endpoint:, endpoint:)
        input = object("ticket" => text(ticket, "ticket"))
        flags = gre(api, sol, token_endpoint, endpoint)
        Native.run("gre-ticket", input, flags) { |value, raw| GreTicketResult.new(value, raw) }
      end

      def inspect
        "#<#{self.class.name} http_timeout=#{http_timeout}>"
      end

      private

      def soap(credentials, endpoint)
        flags = require_kind(credentials, SolCredentials, "credentials").to_flags
        flags.merge("endpoint" => text(endpoint, "endpoint"), "timeout" => http_timeout)
      end

      def gre(api, sol, token_endpoint, endpoint)
        flags = require_kind(api, ApiCredentials, "api").to_flags
        flags.merge(require_kind(sol, SolCredentials, "sol").to_flags,
                    "token-endpoint" => text(token_endpoint, "token_endpoint"),
                    "endpoint" => text(endpoint, "endpoint"),
                    "timeout" => http_timeout)
      end

      def voucher_key(ruc, document_type, series, number)
        raise TypeError, "number must be an Integer, not #{number.class}" unless number.is_a?(Integer)

        { "ruc" => text(ruc, "ruc"), "document_type" => text(document_type, "document_type"),
          "series" => text(series, "series"), "number" => number }
      end

      # The document's JSON text exactly as the caller wrote it, after checking it is one JSON value: amounts are
      # decimal strings and must reach the SDK untouched.
      def document_json(document)
        return JSON.generate(document) if document.is_a?(Hash)

        json = text(document, "document")
        begin
          JSON.parse(json)
        rescue JSON::ParserError => e
          raise ArgumentError, "document is not one JSON value: #{e.message}"
        end
        json
      end

      def object(fields)
        JSON.generate(fields.compact)
      rescue JSON::GeneratorError => e
        raise ArgumentError, e.message
      end

      def text(value, name)
        raise TypeError, "#{name} must be a String, not #{value.class}" unless value.is_a?(String)

        Arguments.utf8(value, name)
      end

      def optional(value, name)
        value.nil? ? nil : text(value, name)
      end

      def require_kind(value, kind, name)
        raise TypeError, "#{name} must be a #{kind.name.split('::').last}, not #{value.class}" unless value.is_a?(kind)

        value
      end

      def date_text(date)
        return date.strftime("%F") if date.is_a?(Date) || date.is_a?(Time)
        return text(date, "issue_date") if date.is_a?(String)

        raise TypeError, "issue_date must be a Date, a Time or a String, not #{date.class}"
      end

      # A Float is refused: it is not an exact amount. BigDecimal is checked by name so that this gem does not load
      # bigdecimal, a bundled (not default) gem since Ruby 3.4.
      def amount_text(total)
        return total.to_s if total.is_a?(Integer)
        return text(total, "total") if total.is_a?(String)
        return total.to_s("F") if defined?(BigDecimal) && total.is_a?(BigDecimal)
        raise ArgumentError, "total must be exact: pass a BigDecimal or a String, not a Float" if total.is_a?(Float)

        raise TypeError, "total must be a BigDecimal, an Integer or a String, not #{total.class}"
      end

      def whole_seconds(value)
        raise TypeError, "http_timeout must be numeric, not #{value.class}" unless value.is_a?(Numeric)

        seconds = value.to_r
        unless seconds.denominator == 1 && seconds.between?(1, MAX_HTTP_TIMEOUT)
          raise ArgumentError, "http_timeout must be whole seconds from 1 to #{MAX_HTTP_TIMEOUT}, not #{value}"
        end

        seconds.to_i
      end
    end
  end
end
