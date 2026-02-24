require_relative "test_helper"

class TestHwarang < Minitest::Test
  FIXTURES = File.expand_path("fixtures", __dir__)
  SAMPLE_HWP = File.join(FIXTURES, "sample.hwp")
  SAMPLE_HWPX = File.join(FIXTURES, "sample.hwpx")

  def test_version
    refute_nil Hwarang::VERSION
  end

  def test_extract_text_hwp
    text = Hwarang.extract_text(SAMPLE_HWP)
    assert_kind_of String, text
    refute_empty text
  end

  def test_extract_text_hwpx
    text = Hwarang.extract_text(SAMPLE_HWPX)
    assert_kind_of String, text
    refute_empty text
  end

  def test_extract_text_file_not_found
    assert_raises(Hwarang::FileError) do
      Hwarang.extract_text("/nonexistent/file.hwp")
    end
  end

  def test_extract_text_invalid_format
    tmpfile = File.join(FIXTURES, "invalid.tmp")
    File.write(tmpfile, "not a hwp file")
    assert_raises(Hwarang::UnsupportedFormatError) do
      Hwarang.extract_text(tmpfile)
    end
  ensure
    File.delete(tmpfile) if tmpfile && File.exist?(tmpfile)
  end

  def test_list_streams
    streams = Hwarang.list_streams(SAMPLE_HWP)
    assert_kind_of Array, streams
    refute_empty streams
    assert_includes streams, "/FileHeader"
  end

  def test_extract_batch
    paths = [SAMPLE_HWP, SAMPLE_HWPX]
    results = Hwarang.extract_batch(paths)
    assert_kind_of Hash, results
    assert_equal 2, results.size

    paths.each do |path|
      result = results[path]
      assert_kind_of Hash, result
      assert result.key?("text") || result.key?("error")
    end
  end

  def test_extract_batch_with_errors
    paths = [SAMPLE_HWP, "/nonexistent/file.hwp"]
    results = Hwarang.extract_batch(paths)
    assert_equal 2, results.size
    assert results[SAMPLE_HWP].key?("text")
    assert results["/nonexistent/file.hwp"].key?("error")
  end

  def test_error_hierarchy
    assert Hwarang::Error < StandardError
    assert Hwarang::FileError < Hwarang::Error
    assert Hwarang::InvalidSignatureError < Hwarang::Error
    assert Hwarang::UnsupportedVersionError < Hwarang::Error
    assert Hwarang::PasswordProtectedError < Hwarang::Error
    assert Hwarang::StreamNotFoundError < Hwarang::Error
    assert Hwarang::InvalidRecordHeaderError < Hwarang::Error
    assert Hwarang::DecompressFailedError < Hwarang::Error
    assert Hwarang::DecryptFailedError < Hwarang::Error
    assert Hwarang::ParseError < Hwarang::Error
    assert Hwarang::UnsupportedFormatError < Hwarang::Error
    assert Hwarang::HwpxError < Hwarang::Error
  end
end
