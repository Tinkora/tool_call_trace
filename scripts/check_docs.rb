# frozen_string_literal: true

require "open3"
require "optparse"
require "set"

REQUIRED_FILES = %w[
  README.md
  README.zh-CN.md
  LICENSE
  CONTRIBUTING.md
  CONTRIBUTING.zh-CN.md
  SECURITY.md
  SECURITY.zh-CN.md
  SUPPORT.md
  SUPPORT.zh-CN.md
  CODE_OF_CONDUCT.md
  CODE_OF_CONDUCT.zh-CN.md
  CHANGELOG.md
  MAINTAINERS.md
  .github/CODEOWNERS
  docs/PRODUCT_SPEC.md
  docs/PRODUCT_SPEC.zh-CN.md
  docs/MATURITY.md
  docs/MATURITY.zh-CN.md
].freeze

BILINGUAL_PAIRS = [
  %w[README.md README.zh-CN.md],
  %w[CONTRIBUTING.md CONTRIBUTING.zh-CN.md],
  %w[SECURITY.md SECURITY.zh-CN.md],
  %w[SUPPORT.md SUPPORT.zh-CN.md],
  %w[CODE_OF_CONDUCT.md CODE_OF_CONDUCT.zh-CN.md],
  %w[docs/PRODUCT_SPEC.md docs/PRODUCT_SPEC.zh-CN.md],
  %w[docs/MATURITY.md docs/MATURITY.zh-CN.md],
  %w[docs/decisions/0001-require-real-timestamps.md docs/decisions/0001-require-real-timestamps.zh-CN.md]
].freeze

TEXT_EXTENSIONS = %w[
  .html .js .json .jsonc .lock .md .mjs .rs .toml .yaml .yml
].freeze
TEXT_FILENAMES = %w[.gitignore LICENSE].freeze
UTF8_BOM = "\xEF\xBB\xBF".b.freeze
FORBIDDEN_PUBLIC_TEXT = Regexp.new(%w[agent com mons].join, Regexp::IGNORECASE)

options = { root: Dir.pwd }
OptionParser.new do |parser|
  parser.on("--root PATH") { |path| options[:root] = path }
end.parse!

root = File.expand_path(options[:root])
errors = []
stdout, stderr, status = Open3.capture3("git", "-C", root, "ls-files", "-z")
unless status.success?
  warn "Unable to list tracked files: #{stderr.strip}"
  exit 1
end
tracked_files = stdout.split("\0").reject(&:empty?).to_set

REQUIRED_FILES.each do |path|
  unless tracked_files.include?(path) && File.file?(File.join(root, path))
    errors << "Missing required file: #{path}"
  end
end

BILINGUAL_PAIRS.each do |english, chinese|
  english_path = File.join(root, english)
  chinese_path = File.join(root, chinese)
  english_exists = tracked_files.include?(english) && File.file?(english_path)
  chinese_exists = tracked_files.include?(chinese) && File.file?(chinese_path)
  errors << "Missing bilingual pair: #{english}" unless english_exists
  errors << "Missing bilingual pair: #{chinese}" unless chinese_exists
  next unless english_exists && chinese_exists

  english_text = File.read(english_path, encoding: "UTF-8", invalid: :replace, undef: :replace)
  chinese_text = File.read(chinese_path, encoding: "UTF-8", invalid: :replace, undef: :replace)
  unless english_text.include?(File.basename(chinese))
    errors << "Missing Chinese entry link in #{english}"
  end
  unless chinese_text.include?(File.basename(english))
    errors << "Missing English entry link in #{chinese}"
  end
end

package_version = nil
if tracked_files.include?("Cargo.toml") && File.file?(File.join(root, "Cargo.toml"))
  cargo_toml = File.read(File.join(root, "Cargo.toml"), encoding: "UTF-8", invalid: :replace, undef: :replace)
  package_version = cargo_toml[/^\s*version\s*=\s*"([^"]+)"/, 1] if cargo_toml.valid_encoding?
end

if package_version
  release_tag = "v#{package_version}"
  %w[README.md README.zh-CN.md].each do |path|
    next unless tracked_files.include?(path) && File.file?(File.join(root, path))

    readme = File.read(File.join(root, path), encoding: "UTF-8", invalid: :replace, undef: :replace)
    unless readme.include?("/releases/tag/#{release_tag}")
      errors << "#{path} does not link to the current release #{release_tag}"
    end
  end
end

text_files = tracked_files.select do |path|
  TEXT_EXTENSIONS.include?(File.extname(path).downcase) ||
    TEXT_FILENAMES.include?(File.basename(path))
end

text_files.sort.each do |path|
  absolute_path = File.join(root, path)
  next unless File.file?(absolute_path)

  bytes = File.binread(absolute_path)
  errors << "UTF-8 BOM is not allowed: #{path}" if bytes.start_with?(UTF8_BOM)
  content = bytes.force_encoding(Encoding::UTF_8)
  if content.valid_encoding?
    errors << "Legacy organization reference is forbidden: #{path}" if content.match?(FORBIDDEN_PUBLIC_TEXT)
  else
    errors << "Invalid UTF-8: #{path}"
  end
rescue SystemCallError => error
  errors << "Unable to read #{path}: #{error.message}"
end

if errors.empty?
  puts "Documentation checks passed (#{text_files.length} tracked text files scanned)."
  exit 0
end

errors.each { |error| warn error }
exit 1
