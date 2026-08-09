# frozen_string_literal: true

require "fileutils"
require "optparse"
require "tmpdir"

REQUIRED_WASM_FILES = %w[
  package.json
  tool_call_trace_web.js
  tool_call_trace_web_bg.wasm
].freeze
OPTIONAL_WASM_FILES = %w[
  .gitignore
  LICENSE
  README.md
  tool_call_trace_web.d.ts
  tool_call_trace_web_bg.wasm.d.ts
].freeze
COPIED_WASM_FILES = (REQUIRED_WASM_FILES + OPTIONAL_WASM_FILES).reject do |name|
  name == ".gitignore"
end.freeze

options = {
  root: File.expand_path("..", __dir__),
  wasm_package: nil
}
OptionParser.new do |parser|
  parser.on("--root PATH") { |path| options[:root] = path }
  parser.on("--wasm-package PATH") { |path| options[:wasm_package] = path }
end.parse!

begin
  root = File.realpath(options[:root])
  source_argument = options[:wasm_package]
  raise "--wasm-package is required" if source_argument.nil? || source_argument.empty?

  source_metadata = File.lstat(source_argument)
  raise "WASM package must be a real directory" unless source_metadata.directory? && !source_metadata.symlink?

  source = File.realpath(source_argument)
  source_entries = Dir.children(source).sort
  source_entries.each do |name|
    path = File.join(source, name)
    metadata = File.lstat(path)
    raise "WASM package contains a symbolic link: #{name}" if metadata.symlink?
    raise "WASM package contains a non-file entry: #{name}" unless metadata.file?
    next if (REQUIRED_WASM_FILES + OPTIONAL_WASM_FILES).include?(name)

    raise "WASM package contains an unexpected file: #{name}"
  end
  REQUIRED_WASM_FILES.each do |name|
    raise "WASM package is missing #{name}" unless source_entries.include?(name)
  end

  static_index = File.join(root, "crates/tool_call_trace_web/static/index.html")
  index_metadata = File.lstat(static_index)
  raise "Static UI must be a real file" unless index_metadata.file? && !index_metadata.symlink?

  output = File.join(root, "dist")
  if File.exist?(output) || File.symlink?(output)
    output_metadata = File.lstat(output)
    raise "dist must be a real directory" unless output_metadata.directory? && !output_metadata.symlink?
  end

  staging = Dir.mktmpdir(".pages-build-", root)
  backup = nil
  begin
    FileUtils.copy_file(static_index, File.join(staging, "index.html"))
    package_output = File.join(staging, "pkg")
    Dir.mkdir(package_output)
    COPIED_WASM_FILES.each do |name|
      source_file = File.join(source, name)
      FileUtils.copy_file(source_file, File.join(package_output, name)) if File.file?(source_file)
    end

    if File.exist?(output)
      backup = Dir.mktmpdir(".pages-previous-", root)
      Dir.rmdir(backup)
      File.rename(output, backup)
    end

    begin
      File.rename(staging, output)
      staging = nil
    rescue StandardError
      File.rename(backup, output) if backup && File.exist?(backup) && !File.exist?(output)
      raise
    end
    FileUtils.remove_entry_secure(backup) if backup && File.exist?(backup)
    puts "Pages artifact assembled in #{output}."
  ensure
    FileUtils.remove_entry_secure(staging) if staging && File.exist?(staging)
  end
rescue OptionParser::ParseError, SystemCallError, RuntimeError => error
  warn error.message
  exit 1
end
