# frozen_string_literal: true

require "fileutils"
require "minitest/autorun"
require "open3"
require "rbconfig"
require "tmpdir"
require "yaml"

class CheckWorkflowContractsTest < Minitest::Test
  CHECKER = File.expand_path("check_workflow_contracts.rb", __dir__)
  COMMIT = "e967aed0860957b24daf57e66766713c60b5bcae"

  def test_valid_tinkora_references_pass
    with_fixture do |root|
      result = run_checker(root)

      assert result[:status].success?, result[:output]
      assert_includes result[:output], "Reusable workflow contracts passed"
    end
  end

  def test_floating_reference_fails
    with_fixture(reference: "main") do |root|
      result = run_checker(root)

      refute result[:status].success?
      assert_includes result[:output], "@#{COMMIT}"
    end
  end

  def test_wrong_owner_fails
    with_fixture(owner: "retired-org") do |root|
      result = run_checker(root)

      refute result[:status].success?
      assert_includes result[:output], "must use Tinkora/.github"
    end
  end

  def test_missing_wasm_job_fails
    with_fixture(include_wasm: false) do |root|
      result = run_checker(root)

      refute result[:status].success?
      assert_includes result[:output], "job wasm must use"
    end
  end

  def test_floating_pages_reference_fails
    with_fixture(pages_reference: "main") do |root|
      result = run_checker(root)

      refute result[:status].success?
      assert_includes result[:output], "reusable-pages.yml@#{COMMIT}"
    end
  end

  def test_pages_deployment_requires_the_main_branch
    with_fixture(pages_main_gate: false) do |root|
      result = run_checker(root)

      refute result[:status].success?
      assert_includes result[:output], "must restrict assembly and deployment to main"
    end
  end

  def test_pages_artifacts_include_the_run_attempt
    with_fixture(pages_run_attempt: false) do |root|
      result = run_checker(root)

      refute result[:status].success?
      assert_includes result[:output], "must include github.run_attempt"
    end
  end

  private

  def with_fixture(
    owner: "Tinkora",
    reference: COMMIT,
    include_wasm: true,
    pages_reference: reference,
    pages_main_gate: true,
    pages_run_attempt: true
  )
    Dir.mktmpdir("workflow-contracts-") do |root|
      quality_jobs = {
        "rust" => "#{owner}/.github/.github/workflows/reusable-rust-quality.yml@#{reference}"
      }
      if include_wasm
        quality_jobs["wasm"] =
          "#{owner}/.github/.github/workflows/reusable-wasm-quality.yml@#{reference}"
      end
      write_workflow(root, ".github/workflows/quality.yml", quality_jobs)
      write_workflow(
        root,
        ".github/workflows/supply-chain.yml",
        "audit" => "#{owner}/.github/.github/workflows/reusable-supply-chain.yml@#{reference}"
      )
      write_pages_workflow(
        root,
        owner: owner,
        reference: pages_reference,
        main_gate: pages_main_gate,
        run_attempt: pages_run_attempt
      )
      yield root
    end
  end

  def write_workflow(root, relative_path, jobs)
    absolute_path = File.join(root, relative_path)
    FileUtils.mkdir_p(File.dirname(absolute_path))
    document = { "name" => "Fixture", "jobs" => jobs.transform_values { |uses| { "uses" => uses } } }
    File.write(absolute_path, YAML.dump(document), encoding: "UTF-8")
  end

  def write_pages_workflow(root, owner:, reference:, main_gate:, run_attempt:)
    suffix = run_attempt ? "-${{ github.run_attempt }}" : ""
    main_condition = main_gate ? "github.ref == 'refs/heads/main'" : "success()"
    document = {
      "name" => "Fixture Pages",
      "jobs" => {
        "assemble" => {
          "if" => main_condition,
          "steps" => [
            { "with" => { "name" => "wasm-package-${{ github.run_id }}#{suffix}" } },
            { "with" => { "name" => "pages-source-${{ github.run_id }}#{suffix}" } }
          ]
        },
        "deploy" => {
          "if" => main_condition,
          "uses" => "#{owner}/.github/.github/workflows/reusable-pages.yml@#{reference}",
          "with" => {
            "source-artifact-name" => "pages-source-${{ github.run_id }}#{suffix}"
          }
        }
      }
    }
    absolute_path = File.join(root, ".github/workflows/pages.yml")
    FileUtils.mkdir_p(File.dirname(absolute_path))
    File.write(absolute_path, YAML.dump(document), encoding: "UTF-8")
  end

  def run_checker(root)
    stdout, stderr, status = Open3.capture3(RbConfig.ruby, CHECKER, "--root", root)
    { output: stdout + stderr, status: status }
  end
end
