# frozen_string_literal: true

require "English"
require "json"
require "optparse"

options = {}
OptionParser.new do |parser|
  parser.on("--tag TAG") { |value| options[:tag] = value }
  parser.on("--notes PATH") { |value| options[:notes] = value }
end.parse!

tag = options.fetch(:tag)
notes_path = options.fetch(:notes)
abort("tag must use the vX.Y.Z form") unless tag.match?(/\Av(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\z/)

version = tag.delete_prefix("v")
metadata_output = `cargo metadata --format-version 1 --no-deps --locked`
abort("cargo metadata failed") unless $CHILD_STATUS.success?
metadata = JSON.parse(metadata_output)
workspace_members = metadata.fetch("workspace_members")
workspace_packages = metadata.fetch("packages").select do |package|
  workspace_members.include?(package.fetch("id"))
end
abort("workspace has no packages") if workspace_packages.empty?
unless workspace_packages.all? { |package| package.fetch("version") == version }
  abort("workspace package versions do not match #{version}")
end

changelog = File.read("CHANGELOG.md", encoding: "UTF-8")
header = /^## \[#{Regexp.escape(version)}\] - \d{4}-\d{2}-\d{2}$/
match = changelog.match(header)
abort("CHANGELOG.md has no #{version} release section") unless match
boundaries = [
  changelog.match(/^## /, match.end(0)),
  changelog.match(/^\[[^\]]+\]:\s+\S+$/, match.end(0))
].compact
boundary_offset = boundaries.map { |boundary| boundary.begin(0) }.min || changelog.length
notes = changelog[match.end(0)...boundary_offset].strip
abort("CHANGELOG.md #{version} section is empty") if notes.empty?
File.write(notes_path, "#{notes}\n", encoding: "UTF-8")
