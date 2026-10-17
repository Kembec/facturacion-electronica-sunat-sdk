# frozen_string_literal: true

# The battery of the gem as a consumer installs it: the 38 fixtures against their vectors (inclusive C14N, exact
# DigestValue and SignatureValue), the native version, zip and CDR, the errors, no_verdict without an answer, concurrent
# calls and the GVL released while a call waits. Exit code 0 only if every check passed.
#
#   ruby vectors.rb <repository> [--c14n-dump DIR]
#
# C14N needs nokogiri. Where it has no binary (32-bit Windows on Ruby 3.4), --c14n-dump writes each built and signed
# XML to DIR and c14n.rb compares them from a Ruby that has it; DigestValue and SignatureValue are still compared here.
# SUNAT_SDK_FROM_SOURCE=1 accepts the library from -I instead of an installed gem (development only).

require "kembec/sunat_sdk"
require "json"
require "socket"

module Vectors
  EXPECTED_FIXTURES = 38
  GEM = "kembec-sunat_sdk"
  SDK = Kembec::SunatSdk

  class Battery
    def initialize(repo, dump)
      @repo = File.expand_path(repo)
      @dump = dump
      @checks = 0
      @failures = 0
      @lock = Mutex.new
      @key = SDK::SigningKey.new(path("tests", "vectors", "test-cert", "test-cert.pem"),
                                 path("tests", "vectors", "test-cert", "test-key.pem"))
      @client = SDK::Client.new(http_timeout: 30)
      @c14n = load_nokogiri
      Dir.mkdir(@dump) if @dump && !Dir.exist?(@dump)
    end

    def run
      fixtures = Dir[path("fixtures", "*.json")].sort
      check(fixtures.size == EXPECTED_FIXTURES, "fixtures: #{fixtures.size}, expected #{EXPECTED_FIXTURES}")
      check(@c14n || @dump, "C14N: nokogiri is not available and there is no --c14n-dump")
      step("version") { version }
      fixtures.each { |fixture| step(File.basename(fixture, ".json")) { vector(fixture) } }
      step("zip") { zip }
      step("cdr") { read_cdr }
      step("errors") { errors }
      step("no_verdict") { no_verdicts }
      step("threads") { threads }
      step("gvl") { gvl }
      report(fixtures.size)
    end

    private

    def report(vectors)
      runtime = "Ruby #{RUBY_VERSION} #{RUBY_PLATFORM}, #{FFI::Platform::ADDRESS_SIZE}-bit, ffi #{FFI::VERSION}"
      c14n = @c14n ? "C14N by nokogiri #{Nokogiri::VERSION}" : "C14N dumped to #{@dump}"
      if @failures.positive?
        warn "Ruby: #{@failures} of #{@checks} checks failed (#{runtime})"
        return 1
      end
      puts "ok Ruby: #{@checks} checks, #{vectors} vectors, #{runtime}, #{c14n}, native #{SDK.native_version}"
      0
    end

    # The gem that runs is the installed one, for this platform, with its own library: never the source tree.
    def version
      check(SDK.native_version == SDK::SDK_VERSION, "native #{SDK.native_version}, gem #{SDK::SDK_VERSION}")
      return if ENV["SUNAT_SDK_FROM_SOURCE"] == "1"

      spec = Gem.loaded_specs[GEM]
      check(!spec.nil?, "#{GEM} was not loaded as a gem")
      return if spec.nil?

      check(spec.version.to_s == SDK::VERSION, "gem version #{spec.version}, library #{SDK::VERSION}")
      expected = ENV.fetch("EXPECT_PLATFORM", "")
      check(expected.empty? || spec.platform.to_s == expected, "gem platform #{spec.platform}, expected #{expected}")
      library = SDK::Native.library_path
      check(library.start_with?(spec.full_gem_path) && File.file?(library), "library #{library} is not the gem's")
      bits = ENV.fetch("EXPECT_ADDRESS_SIZE", "")
      check(bits.empty? || FFI::Platform::ADDRESS_SIZE.to_s == bits, "#{FFI::Platform::ADDRESS_SIZE}-bit process")
    end

    def vector(fixture)
      name = File.basename(fixture, ".json")
      document = File.read(fixture, encoding: "UTF-8")
      built = @client.build(document)
      check(built.file_name == expected_file_name(JSON.parse(document)), "#{name}: file name #{built.file_name}")
      signed = @client.sign(built.xml, @key)
      expected = read_vector(name, "signed")
      check(expected.include?("<ds:DigestValue>#{signed.digest_value}</ds:DigestValue>"),
            "#{name}: DigestValue #{signed.digest_value}")
      check(expected.include?("<ds:SignatureValue>#{signed.signature_value}</ds:SignatureValue>"),
            "#{name}: SignatureValue differs")
      check(signed.xml.include?("<ds:DigestValue>#{signed.digest_value}</ds:DigestValue>"),
            "#{name}: DigestValue not in the signed XML")
      compare(name, "unsigned", built.xml)
      compare(name, "signed", signed.xml)
    end

    def compare(name, kind, xml)
      File.binwrite(File.join(@dump, "#{name}.#{kind}.xml"), xml) if @dump
      return unless @c14n

      check(canonical(read_vector(name, kind)) == canonical(xml), "#{name}: #{kind} XML differs from its vector under C14N")
    end

    # A Hash document signs exactly as its JSON text; the zip carries the signed XML.
    def zip
      document = JSON.parse(File.read(path("fixtures", "01-invoice-taxed-cash.json"), encoding: "UTF-8"))
      signed = @client.sign(@client.build(document).xml, @key)
      check(signed.digest_value == "+te61a2ub5MlTFf8tKTPNGWEXKI=", "zip: a Hash document signs as its JSON text")
      zip = @client.zip("20123456789-01-F001-1", signed.xml)
      check(zip.file_name == "20123456789-01-F001-1.zip", "zip: file name #{zip.file_name}")
      check(zip.content.encoding == Encoding::BINARY && zip.content.start_with?("PK"), "zip: not a zip")
    end

    def read_cdr
      bytes = File.binread(path("crates", "sunat-transport", "tests", "data", "R-20150147718-01-FT01-982.zip"))
      cdr = @client.read_cdr(bytes)
      check(cdr.reference_id == "FT01-982", "cdr: reference #{cdr.reference_id}")
      check(cdr.response_code == "2047", "cdr: code #{cdr.response_code}")
      check(cdr.response_class == :rejected && !cdr.valid_document?, "cdr: class #{cdr.response_class}")
    end

    def errors
      expect(SDK::Error, "build of {} is the SDK's error") { @client.build("{}") }.then do |e|
        check(e.nil? || e.command == "build", "build of {}: command #{e&.command}")
      end
      expect(ArgumentError, "invalid JSON is an ArgumentError") { @client.build("{not json") }
      expect(TypeError, "a nil document is a TypeError") { @client.build(nil) }
      expect(ArgumentError, "timeout of 1.5 s") { SDK::Client.new(http_timeout: 1.5) }
      expect(ArgumentError, "a Float total") { @client.validity(**voucher, issue_date: "2026-10-01", total: 118.0, credentials: api) }
      sol = SDK::SolCredentials.new("20123456789MODDATOS", "moddatos-secret")
      check(!sol.inspect.include?("moddatos-secret") && !sol.to_s.include?("moddatos-secret"), "SolCredentials shows the password")
      signed = @client.sign(@client.build(File.read(path("fixtures", "01-invoice-taxed-cash.json"))).xml, @key).xml
      error = expect(SDK::Error, "send over http://") do
        @client.send_bill("20123456789-01-F001-1", signed, credentials: sol, endpoint: "http://127.0.0.1/billService")
      end
      check(error.nil? || (error.command == "send" && error.message.include?("endpoint must use https://") &&
                           !error.message.include?("moddatos-secret")), "send over http://: #{error&.message}")
    end

    # A listener that accepts and never answers the TLS handshake: every query waits out its 1 s.
    def no_verdicts
      silent do |base|
        client = SDK::Client.new(http_timeout: 1)
        sol = SDK::SolCredentials.new("20123456789MODDATOS", "moddatos")
        status = client.status(**voucher, credentials: sol, endpoint: "#{base}/billConsultService")
        check(status.outcome == :no_verdict && status.no_verdict.reason == :timeout && status.no_verdict.http_status.nil?,
              "status without an answer: #{status.raw_json}")
        validity = client.validity(**voucher, issue_date: Date.new(2026, 10, 1), total: "118.00", credentials: api,
                                              token_endpoint: "#{base}/token", endpoint: "#{base}/query")
        check(validity.outcome == :no_verdict && validity.no_verdict.stage == :token, "validity without a token: #{validity.raw_json}")
        gre = client.gre_ticket("1700000000000", api: api, sol: sol, token_endpoint: "#{base}/token", endpoint: "#{base}/v1")
        check(gre.outcome == :no_verdict && gre.no_verdict.stage == :token && gre.ticket == "1700000000000",
              "gre-ticket without a token: #{gre.raw_json}")
      end
    end

    # Eight threads build and sign different fixtures at once; each must still match its vector.
    def threads
      names = Dir[path("fixtures", "*.json")].map { |f| File.basename(f, ".json") }.sort.first(8)
      results = {}
      workers = names.map do |name|
        Thread.new do
          document = File.read(path("fixtures", "#{name}.json"), encoding: "UTF-8")
          5.times { results[name] = @client.sign(@client.build(document).xml, @key).digest_value }
        end
      end
      workers.each(&:join)
      names.each do |name|
        check(read_vector(name, "signed").include?("<ds:DigestValue>#{results[name]}</ds:DigestValue>"), "threads: #{name}")
      end
    end

    # While a query waits 1 s on a silent server, the main thread keeps running: the call released the GVL.
    # A held GVL gives 1 tick; released, a slow CI runner gave 19 (macOS x86_64, Ruby 3.3), so the bound is 5.
    def gvl
      silent do |base|
        client = SDK::Client.new(http_timeout: 1)
        sol = SDK::SolCredentials.new("20123456789MODDATOS", "moddatos")
        waiting = Thread.new { client.status(**voucher, credentials: sol, endpoint: "#{base}/billConsultService") }
        ticks = 0
        while waiting.alive?
          ticks += 1
          sleep 0.01
        end
        check(waiting.value.outcome == :no_verdict, "gvl: the query answered #{waiting.value.raw_json}")
        check(ticks >= 5, "gvl: the main thread ran #{ticks} times in a 1 s call")
      end
    end

    def silent
      server = TCPServer.new("127.0.0.1", 0)
      held = []
      acceptor = Thread.new { loop { held << server.accept } }
      yield "https://127.0.0.1:#{server.addr[1]}"
    ensure
      acceptor&.kill
      server&.close
      held&.each(&:close)
    end

    def voucher
      { ruc: "20123456789", document_type: "01", series: "F001", number: 1 }
    end

    def api
      SDK::ApiCredentials.new("client-id", "client-secret")
    end

    # ruc-type-series-number, or ruc-RC|RA-YYYYMMDD-number, from the fixture's own fields.
    def expected_file_name(doc)
      ruc = doc.dig("issuer", "ruc")
      case doc["kind"]
      when "summary" then "#{ruc}-RC-#{doc['generation_date'].delete('-')}-#{doc['number']}"
      when "voided" then "#{ruc}-RA-#{doc['communication_date'].delete('-')}-#{doc['number']}"
      else "#{ruc}-#{doc['document_type']}-#{doc['series']}-#{doc['number']}"
      end
    end

    def canonical(xml)
      Nokogiri::XML(xml) { |config| config.strict.nonet }.canonicalize(Nokogiri::XML::XML_C14N_1_0)
    end

    def load_nokogiri
      require "nokogiri"
      true
    rescue LoadError
      false
    end

    def read_vector(name, kind)
      File.read(path("tests", "vectors", "#{name}.#{kind}.xml"), encoding: "UTF-8")
    end

    def path(*parts)
      File.join(@repo, *parts)
    end

    # Runs the block expecting exactly that error class; returns the error, or nil.
    def expect(kind, what)
      yield
      check(false, "#{what}: no error")
      nil
    rescue StandardError => e
      check(e.instance_of?(kind), "#{what}: #{e.class} #{e.message}")
      e.instance_of?(kind) ? e : nil
    end

    def step(what)
      yield
    rescue StandardError => e
      check(false, "#{what}: #{e.class} #{e.message}")
    end

    def check(ok, what)
      @lock.synchronize do
        @checks += 1
        next if ok

        @failures += 1
        warn "FAIL #{what}"
      end
    end
  end
end

if ARGV.empty? || ARGV.size == 2 || (ARGV.size == 3 && ARGV[1] != "--c14n-dump") || ARGV.size > 3
  warn "usage: ruby vectors.rb <repository> [--c14n-dump DIR]"
  exit 2
end
exit Vectors::Battery.new(ARGV[0], ARGV[2]).run
