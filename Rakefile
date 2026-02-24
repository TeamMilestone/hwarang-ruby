require "bundler/gem_tasks"
require "rake/testtask"
require "rake/extensiontask"

Rake::TestTask.new do |t|
  t.test_files = FileList["test/**/test_*.rb"]
end

gemspec = Bundler.load_gemspec("hwarang.gemspec")
Rake::ExtensionTask.new("hwarang", gemspec) do |ext|
  ext.lib_dir = "lib/hwarang"
end

task default: :test
