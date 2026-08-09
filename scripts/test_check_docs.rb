# frozen_string_literal: true

require "fileutils"
require "minitest/autorun"
require "open3"
require "rbconfig"
require "tmpdir"

class CheckDocsTest < Minitest::Test
  CHECKER = File.expand_path("check_docs.rb", __dir__)
  PAIRS = [
    %w[README.md README.zh-CN.md],
    %w[CONTRIBUTING.md CONTRIBUTING.zh-CN.md],
    %w[SECURITY.md SECURITY.zh-CN.md],
    %w[SUPPORT.md SUPPORT.zh-CN.md],
    %w[CODE_OF_CONDUCT.md CODE_OF_CONDUCT.zh-CN.md],
    %w[docs/PRODUCT_SPEC.md docs/PRODUCT_SPEC.zh-CN.md],
    %w[docs/MATURITY.md docs/MATURITY.zh-CN.md],
    %w[docs/decisions/0001-require-real-timestamps.md docs/decisions/0001-require-real-timestamps.zh-CN.md]
  ].freeze

  def test_valid_repository_passes
    with_fixture do |root|
      result = run_checker(root)

      assert result[:status].success?, result[:output]
      assert_includes result[:output], "Documentation checks passed"
    end
  end

  def test_missing_required_file_fails
    with_fixture(remove: ["LICENSE"]) do |root|
      result = run_checker(root)

      refute result[:status].success?
      assert_includes result[:output], "Missing required file: LICENSE"
    end
  end

  def test_missing_language_entry_link_fails
    with_fixture(overrides: { "README.md" => "# Project\n" }) do |root|
      result = run_checker(root)

      refute result[:status].success?
      assert_includes result[:output], "Missing Chinese entry link in README.md"
    end
  end

  def test_legacy_organization_reference_fails
    legacy_name = ["Agent", "Commons history\n"].join
    with_fixture(overrides: { "CHANGELOG.md" => "# #{legacy_name}" }) do |root|
      result = run_checker(root)

      refute result[:status].success?
      assert_includes result[:output], "Legacy organization reference is forbidden"
    end
  end

  def test_utf8_bom_fails
    with_fixture(overrides: { "CHANGELOG.md" => "\xEF\xBB\xBF# Changelog\n".b }) do |root|
      result = run_checker(root)

      refute result[:status].success?
      assert_includes result[:output], "UTF-8 BOM is not allowed: CHANGELOG.md"
    end
  end

  def test_invalid_utf8_fails
    with_fixture(overrides: { "Cargo.toml" => "[workspace]\n\xFF".b }) do |root|
      result = run_checker(root)

      refute result[:status].success?
      assert_includes result[:output], "Invalid UTF-8: Cargo.toml"
    end
  end

  private

  def with_fixture(remove: [], overrides: {})
    Dir.mktmpdir("check-docs-") do |root|
      files = fixture_files.merge(overrides)
      remove.each { |path| files.delete(path) }
      files.each do |path, content|
        absolute_path = File.join(root, path)
        FileUtils.mkdir_p(File.dirname(absolute_path))
        File.binwrite(absolute_path, content)
      end
      run_git(root, "init", "--quiet")
      run_git(root, "add", "--all")
      yield root
    end
  end

  def fixture_files
    files = {
      "LICENSE" => "MIT License\n",
      "CHANGELOG.md" => "# Changelog\n",
      "Cargo.toml" => "[workspace]\n",
      "MAINTAINERS.md" => "# Maintainers\n",
      ".github/CODEOWNERS" => "* @maintainer\n"
    }
    PAIRS.each do |english, chinese|
      files[english] = "# English\n\n[Chinese](#{File.basename(chinese)})\n"
      files[chinese] = "# Chinese\n\n[English](#{File.basename(english)})\n"
    end
    files
  end

  def run_checker(root)
    stdout, stderr, status = Open3.capture3(RbConfig.ruby, CHECKER, "--root", root)
    { output: stdout + stderr, status: status }
  end

  def run_git(root, *arguments)
    _stdout, stderr, status = Open3.capture3("git", "-C", root, *arguments)
    raise stderr unless status.success?
  end
end
