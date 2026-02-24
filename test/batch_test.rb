#!/usr/bin/env ruby
require_relative "../lib/hwarang"

DATA_DIR = "/Users/wonsup-mini/projects/superboard/data/raw/files"

# Collect all HWP/HWPX files
files = Dir.glob(File.join(DATA_DIR, "**", "*.{hwp,hwpx,HWP,HWPX}"))
puts "Found #{files.size} files"

start = Process.clock_gettime(Process::CLOCK_MONOTONIC)
results = Hwarang.extract_batch(files)
elapsed = Process.clock_gettime(Process::CLOCK_MONOTONIC) - start

success = 0
failed = 0
errors = Hash.new(0)

results.each do |path, result|
  if result.key?("text")
    success += 1
  else
    failed += 1
    errors[result["error"]] += 1
  end
end

puts "Done: #{success}/#{files.size} succeeded, #{failed} failed, #{elapsed.round(2)}s (#{(files.size / elapsed).round(0)} files/s)"
puts
puts "Error breakdown:"
errors.sort_by { |_, count| -count }.each do |error, count|
  puts "  #{count}\t#{error}"
end
