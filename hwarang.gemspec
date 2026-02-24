require_relative "lib/hwarang/version"

Gem::Specification.new do |spec|
  spec.name = "hwarang"
  spec.version = Hwarang::VERSION
  spec.summary = "Fast HWP/HWPX document text extractor"
  spec.description = "Ruby bindings for the hwarang Rust library. Extracts text from HWP and HWPX documents."
  spec.homepage = "https://github.com/teammilestone/hwarang-ruby"
  spec.license = "MIT"

  spec.author = "Lee Wonsup"
  spec.email = "onesup.lee@gmail.com"

  spec.files = Dir["*.{md,txt}", "{ext,lib}/**/*", "Cargo.*"]
  spec.require_path = "lib"
  spec.extensions = ["ext/hwarang/extconf.rb"]

  spec.required_ruby_version = ">= 3.1"

  spec.add_dependency "rb_sys"
end
