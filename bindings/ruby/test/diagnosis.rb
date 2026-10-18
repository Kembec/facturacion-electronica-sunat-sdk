# frozen_string_literal: true

# The diagnosis of a rejected send, read by the four SOAP results from a response the core wrote: the beta's real 2800
# answer (the recorded body in fe-kembec's cierre/datos/rechazado.json) run through the commands' exchange and diagnosis,
# kept in fixtures/send-fault-2800.json. Runnable alone (`ruby -Ilib test/diagnosis.rb <repository>`, no library needed)
# and from vectors.rb, which gives its own `check`.

require "kembec/sunat_sdk"
require "json"

module DiagnosisChecks
  SDK = Kembec::SunatSdk
  FIXTURE = File.join(__dir__, "fixtures", "send-fault-2800.json")

  module_function

  # Calls check(ok, what) for each assertion.
  def run(check)
    raw = File.read(FIXTURE, encoding: "UTF-8")
    value = JSON.parse(raw)
    { SDK::SendResult => "send", SDK::SummaryResult => "summary", SDK::TicketResult => "ticket",
      SDK::StatusResult => "status" }.each do |kind, name|
      read_2800(check, name, kind.new(value, raw))
      empty = kind.new(value.reject { |key, _| key == "diagnosis" }, raw)
      check.call(empty.diagnosis.empty? && empty.diagnosis.frozen?, "diagnosis #{name}: absent is an empty list")
    end
    check.call(SDK::SendResult.new(value, raw).fault.code == "2800", "diagnosis: the fault is still read")
  end

  def read_2800(check, name, result)
    diagnosis = result.diagnosis
    check.call(diagnosis.size == 1 && diagnosis.frozen?, "diagnosis #{name}: #{diagnosis.size} elements")
    first = diagnosis.first
    check.call(first&.code == "2800", "diagnosis #{name}: code #{first&.code}")
    detail = first&.detail
    check.call(detail&.node == "cbc:ID/schemeID" && detail&.value == "1", "diagnosis #{name}: detail #{detail.inspect}")
    field = first&.fields&.first
    check.call(first&.fields&.size == 1 && field.input_path == "customer.document_type" && field.certainty == "fuente" &&
               field.xpath.end_with?("cbc:ID@schemeID"), "diagnosis #{name}: field #{field.inspect}")
  end
end

if $PROGRAM_NAME == __FILE__
  failures = 0
  checks = 0
  DiagnosisChecks.run(lambda do |ok, what|
    checks += 1
    next if ok

    failures += 1
    warn "FAIL #{what}"
  end)
  puts "ok Ruby diagnosis: #{checks} checks" if failures.zero?
  exit(failures.zero? ? 0 : 1)
end
