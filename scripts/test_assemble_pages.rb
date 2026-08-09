# frozen_string_literal: true

require "fileutils"
require "minitest/autorun"
require "open3"
require "rbconfig"
require "tmpdir"

class AssemblePagesTest < Minitest::Test
  ASSEMBLER = File.expand_path("assemble_pages.rb", __dir__)

  def test_assembles_the_static_ui_and_wasm_package
    with_fixture do |root, wasm_package|
      result = run_assembler(root, wasm_package)

      assert result[:status].success?, result[:output]
      assert_equal "<!doctype html>\n<title>Tool Call Trace</title>\n",
        File.read(File.join(root, "dist/index.html"), encoding: "UTF-8")
      assert_equal "export default {};\n",
        File.read(File.join(root, "dist/pkg/tool_call_trace_web.js"), encoding: "UTF-8")
      assert File.file?(File.join(root, "dist/pkg/tool_call_trace_web_bg.wasm"))
      refute File.exist?(File.join(root, "dist/pkg/.gitignore"))
      refute File.exist?(File.join(root, "dist/sentinel.txt"))
    end
  end

  def test_rejects_a_symlinked_output_directory
    with_fixture do |root, wasm_package|
      FileUtils.rm_r(File.join(root, "dist"))
      FileUtils.mkdir_p(File.join(root, "outside"))
      File.symlink(File.join(root, "outside"), File.join(root, "dist"))

      result = run_assembler(root, wasm_package)

      refute result[:status].success?
      assert_includes result[:output], "dist must be a real directory"
      assert_empty Dir.children(File.join(root, "outside"))
    end
  end

  def test_rejects_symlinks_without_replacing_the_previous_site
    with_fixture do |root, wasm_package|
      File.write(File.join(root, "dist/sentinel.txt"), "previous\n", encoding: "UTF-8")
      File.symlink("tool_call_trace_web.js", File.join(wasm_package, "linked.js"))

      result = run_assembler(root, wasm_package)

      refute result[:status].success?
      assert_includes result[:output], "symbolic link"
      assert_equal "previous\n",
        File.read(File.join(root, "dist/sentinel.txt"), encoding: "UTF-8")
    end
  end

  private

  def with_fixture
    Dir.mktmpdir("assemble-pages-") do |root|
      static = File.join(root, "crates/tool_call_trace_web/static")
      wasm_package = File.join(root, "wasm-package")
      FileUtils.mkdir_p(static)
      FileUtils.mkdir_p(wasm_package)
      FileUtils.mkdir_p(File.join(root, "dist"))
      File.write(File.join(root, "dist/sentinel.txt"), "previous\n", encoding: "UTF-8")
      File.write(
        File.join(static, "index.html"),
        "<!doctype html>\n<title>Tool Call Trace</title>\n",
        encoding: "UTF-8"
      )
      File.write(File.join(wasm_package, "package.json"), "{}\n", encoding: "UTF-8")
      File.write(
        File.join(wasm_package, "tool_call_trace_web.js"),
        "export default {};\n",
        encoding: "UTF-8"
      )
      File.binwrite(File.join(wasm_package, "tool_call_trace_web_bg.wasm"), "\0asm")
      File.write(File.join(wasm_package, ".gitignore"), "*\n", encoding: "UTF-8")
      yield root, wasm_package
    end
  end

  def run_assembler(root, wasm_package)
    stdout, stderr, status = Open3.capture3(
      RbConfig.ruby,
      ASSEMBLER,
      "--root",
      root,
      "--wasm-package",
      wasm_package
    )
    { output: stdout + stderr, status: status }
  end
end
