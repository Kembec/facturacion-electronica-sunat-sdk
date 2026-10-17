# frozen_string_literal: true

# The C14N half of the battery for a Ruby where nokogiri has no binary (32-bit Windows on Ruby 3.4): compares the XML
# that `vectors.rb --c14n-dump DIR` wrote, built and signed by the gem in that Ruby, with the vectors under inclusive
# C14N 1.0. Runs on any Ruby with nokogiri. Exit code 0 only if all 76 documents match.
#
#   ruby c14n.rb <repository> <DIR>

require "nokogiri"

if ARGV.size != 2
  warn "usage: ruby c14n.rb <repository> <DIR>"
  exit 2
end
repo, dump = ARGV
canonical = lambda do |file|
  Nokogiri::XML(File.read(file, encoding: "UTF-8")) { |config| config.strict.nonet }
          .canonicalize(Nokogiri::XML::XML_C14N_1_0)
end
names = Dir[File.join(repo, "fixtures", "*.json")].map { |f| File.basename(f, ".json") }.sort
failures = 0
names.product(%w[unsigned signed]).each do |name, kind|
  produced = File.join(dump, "#{name}.#{kind}.xml")
  vector = File.join(repo, "tests", "vectors", "#{name}.#{kind}.xml")
  begin
    next if File.file?(produced) && canonical.call(produced) == canonical.call(vector)
  rescue Nokogiri::XML::SyntaxError => e
    warn "#{name}: #{kind} XML is not XML: #{e.message}"
  end

  failures += 1
  warn "FAIL #{name}: #{kind} XML #{File.file?(produced) ? 'differs from its vector under C14N' : 'was not dumped'}"
end
if names.size != 38 || failures.positive?
  warn "C14N: #{failures} of #{names.size * 2} documents failed (#{names.size} fixtures)"
  exit 1
end
puts "ok C14N: #{names.size * 2} documents of #{names.size} vectors match, nokogiri #{Nokogiri::VERSION}"
