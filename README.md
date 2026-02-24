# hwarang

HWP/HWPX 문서에서 텍스트를 빠르게 추출하는 Ruby gem입니다.

Rust로 작성된 [hwarang](https://crates.io/crates/hwarang) 크레이트의 Ruby 바인딩입니다.

## 지원 포맷

- **HWP** (OLE 바이너리) - 한/글 5.x 이상
- **HWPX** (ZIP/XML) - 한/글 최신 XML 기반 포맷
- **HWPML** (순수 XML)

## 주요 기능

- 매직 바이트 기반 포맷 자동 감지
- 압축/비압축 스트림 모두 지원
- 배포문서 복호화 (AES/ECB)
- 표를 마크다운 테이블로 변환
- 머리글/꼬리글, 각주/미주, 글상자, 숨은설명 추출
- rayon 기반 병렬 배치 처리

## 설치

```ruby
gem install hwarang
```

또는 Gemfile에 추가:

```ruby
gem "hwarang"
```

주요 플랫폼(x86_64-linux, aarch64-linux, arm64-darwin)에는 프리컴파일된 네이티브 gem이 제공됩니다. 그 외 플랫폼에서는 소스 gem이 설치되며 Rust 툴체인이 필요합니다.

## 사용법

### 텍스트 추출

```ruby
require "hwarang"

text = Hwarang.extract_text("document.hwp")
puts text
```

### OLE 스트림 목록

```ruby
streams = Hwarang.list_streams("document.hwp")
# => ["/FileHeader", "/BodyText/Section0", ...]
```

### 배치 처리

여러 파일을 병렬로 처리합니다:

```ruby
paths = Dir.glob("documents/**/*.hwp")
results = Hwarang.extract_batch(paths)

results.each do |path, result|
  if result.key?("text")
    puts "#{path}: #{result["text"].length} chars"
  else
    puts "#{path}: ERROR - #{result["error"]}"
  end
end
```

## 에러 처리

모든 예외는 `Hwarang::Error`를 상속합니다:

```ruby
begin
  Hwarang.extract_text("file.hwp")
rescue Hwarang::PasswordProtectedError
  puts "암호가 걸린 문서입니다"
rescue Hwarang::FileError => e
  puts "파일 오류: #{e.message}"
rescue Hwarang::Error => e
  puts "처리 오류: #{e.message}"
end
```

| 예외 클래스 | 설명 |
|-------------|------|
| `Hwarang::Error` | 기본 예외 클래스 |
| `Hwarang::FileError` | 파일 I/O 오류 |
| `Hwarang::InvalidSignatureError` | HWP 파일 시그니처 불일치 |
| `Hwarang::UnsupportedVersionError` | 지원하지 않는 HWP 버전 |
| `Hwarang::PasswordProtectedError` | 암호 보호된 문서 |
| `Hwarang::StreamNotFoundError` | OLE 스트림 없음 |
| `Hwarang::InvalidRecordHeaderError` | 레코드 헤더 파싱 실패 |
| `Hwarang::DecompressFailedError` | 스트림 압축 해제 실패 |
| `Hwarang::DecryptFailedError` | 복호화 실패 |
| `Hwarang::ParseError` | 일반 파싱 오류 |
| `Hwarang::UnsupportedFormatError` | 지원하지 않는 파일 형식 |
| `Hwarang::HwpxError` | HWPX 처리 오류 |

## 벤치마크

| 항목 | 결과 |
|------|------|
| 파일 수 | 49,353개 (HWP/HWPX) |
| 총 용량 | 1.0 GB |
| 소요 시간 | 43.27초 |
| 처리 속도 | 1,140 files/s |
| 성공률 | 99.94% (49,321/49,353) |
| 환경 | Apple M1, 16GB RAM, 8코어, Ruby 4.0 |

## License

MIT
