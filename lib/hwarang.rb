require_relative "hwarang/version"

begin
  require "hwarang/#{RUBY_VERSION.to_f}/hwarang"
rescue LoadError
  require "hwarang/hwarang"
end
