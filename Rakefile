require "bundler/gem_tasks"
require "rake/testtask"
require "rake/extensiontask"

Rake::TestTask.new do |t|
  t.test_files = FileList["test/**/test_*.rb"]
end

PLATFORMS = %w[
  x86_64-linux
  aarch64-linux
  x86_64-darwin
  arm64-darwin
].freeze

gemspec = Bundler.load_gemspec("hwarang.gemspec")
Rake::ExtensionTask.new("hwarang", gemspec) do |ext|
  ext.lib_dir = "lib/hwarang"
  ext.cross_compile = true
  ext.cross_platform = PLATFORMS
  ext.cross_compiling do |spec|
    spec.dependencies.reject! { |dep| dep.name == "rb_sys" }
    spec.files.reject! { |file| File.fnmatch?("ext/**/*", file) }
  end
end

task default: :test
