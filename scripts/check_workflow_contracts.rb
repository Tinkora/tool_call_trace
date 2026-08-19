# frozen_string_literal: true

require "optparse"
require "yaml"

REUSABLE_WORKFLOW_COMMIT = "ed1ae1d6e3a5f1887f415f985836bec954d1ed41"
PAGES_MAIN_CONDITION = "github.ref == 'refs/heads/main'"
PAGES_WASM_ARTIFACT = "wasm-package-${{ github.run_id }}"
PAGES_SOURCE_ARTIFACT = "pages-source-${{ github.run_id }}"
EXPECTED_CALLS = {
  ".github/workflows/quality.yml" => {
    "rust" => "Tinkora/.github/.github/workflows/reusable-rust-quality.yml@#{REUSABLE_WORKFLOW_COMMIT}",
    "wasm" => "Tinkora/.github/.github/workflows/reusable-wasm-quality.yml@#{REUSABLE_WORKFLOW_COMMIT}"
  },
  ".github/workflows/supply-chain.yml" => {
    "audit" => "Tinkora/.github/.github/workflows/reusable-supply-chain.yml@#{REUSABLE_WORKFLOW_COMMIT}"
  },
  ".github/workflows/pages.yml" => {
    "deploy" => "Tinkora/.github/.github/workflows/reusable-pages.yml@#{REUSABLE_WORKFLOW_COMMIT}"
  }
}.freeze

options = { root: Dir.pwd }
OptionParser.new do |parser|
  parser.on("--root PATH") { |path| options[:root] = path }
end.parse!

def string_values(value)
  case value
  when Hash
    value.values.flat_map { |child| string_values(child) }
  when Array
    value.flat_map { |child| string_values(child) }
  when String
    [value]
  else
    []
  end
end

root = File.expand_path(options[:root])
errors = []

EXPECTED_CALLS.each do |relative_path, expected_jobs|
  workflow_path = File.join(root, relative_path)
  unless File.file?(workflow_path)
    errors << "Missing workflow: #{relative_path}"
    next
  end

  begin
    workflow = YAML.safe_load_file(workflow_path, aliases: false)
    jobs = workflow.fetch("jobs")
    expected_jobs.each do |job_name, expected_reference|
      actual_reference = jobs.dig(job_name, "uses")
      next if actual_reference == expected_reference

      errors << "#{relative_path} job #{job_name} must use #{expected_reference}"
    end

    next unless relative_path == ".github/workflows/pages.yml"

    unless %w[assemble deploy].all? { |job_name| jobs.dig(job_name, "if") == PAGES_MAIN_CONDITION }
      errors << "#{relative_path} must restrict assembly and deployment to main"
    end

    job_values = string_values(jobs)
    unless job_values.include?(PAGES_WASM_ARTIFACT) &&
        job_values.count(PAGES_SOURCE_ARTIFACT) >= 2 &&
        job_values.none? { |value| value.include?("github.run_attempt") }
      errors << "#{relative_path} artifact names must use stable github.run_id values"
    end
  rescue KeyError, Psych::Exception => error
    errors << "Invalid workflow #{relative_path}: #{error.message}"
  end
end

if errors.empty?
  puts "Reusable workflow contracts passed (commit #{REUSABLE_WORKFLOW_COMMIT})."
  exit 0
end

errors.each { |error| warn error }
exit 1
