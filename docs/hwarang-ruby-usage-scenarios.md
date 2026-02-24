# Hwarang Ruby Gem 사용 시나리오

> Ruby에서 hwarang을 어떻게 사용하게 될지에 대한 시나리오.

## 설치

```ruby
# Gemfile
gem 'hwarang'
```

```bash
gem install hwarang
# → Rust 소스가 gem에 포함되어 있어 설치 시 자동 컴파일
# → brew 등 별도 설치 불필요
```

---

## 시나리오 1: 단일 파일 텍스트 추출 (가장 기본)

```ruby
require 'hwarang'

# HWP/HWPX/HWPML 자동 감지
text = Hwarang.extract_text("보고서.hwp")
puts text

# 또는 객체 방식
doc = Hwarang::Document.new("보고서.hwp")
puts doc.text
puts doc.format  #=> :hwp, :hwpx, :hwpml
```

**대응하는 Rust API**: `extract_text_from_file(path) -> Result<String>`

---

## 시나리오 2: 에러 처리

```ruby
begin
  text = Hwarang.extract_text("암호문서.hwp")
rescue Hwarang::PasswordProtectedError
  puts "암호가 걸린 문서입니다"
rescue Hwarang::InvalidSignatureError
  puts "올바른 HWP 파일이 아닙니다"
rescue Hwarang::UnsupportedVersionError => e
  puts "지원하지 않는 버전: #{e.version}"
rescue Hwarang::Error => e
  puts "추출 실패: #{e.message}"
end
```

**대응하는 Rust API**: `HwpError` enum → Ruby 예외 클래스 매핑

---

## 시나리오 3: 대량 파일 배치 처리

```ruby
require 'hwarang'

# 디렉토리의 모든 HWP 파일 처리
hwp_files = Dir.glob("documents/**/*.{hwp,hwpx}")

# 병렬 처리 (Rust의 rayon 활용)
results = Hwarang.extract_batch(hwp_files)
# => { "문서1.hwp" => "텍스트...", "문서2.hwpx" => "텍스트...", ... }

# 또는 블록으로 하나씩 처리
Hwarang.extract_batch(hwp_files) do |path, result|
  case result
  when String
    File.write("#{path}.txt", result)
  when Hwarang::Error
    warn "#{path}: #{result.message}"
  end
end
```

**대응하는 Rust API**: rayon 기반 병렬 처리를 Ruby에 노출

---

## 시나리오 4: Rails에서 검색 인덱싱

```ruby
# app/models/document.rb
class Document < ApplicationRecord
  after_create :index_content

  private

  def index_content
    return unless file.attached?
    return unless file.filename.to_s.match?(/\.(hwp|hwpx)$/i)

    # ActiveStorage에서 파일 다운로드 후 텍스트 추출
    file.open do |tempfile|
      self.update(
        extracted_text: Hwarang.extract_text(tempfile.path)
      )
    end
  end
end
```

---

## 시나리오 5: IO 객체에서 직접 읽기

```ruby
# 파일 경로 대신 IO 객체 전달
File.open("보고서.hwp", "rb") do |f|
  text = Hwarang.extract_text(f)
  puts text
end

# StringIO에서도 가능 (네트워크로 받은 데이터 등)
data = download_from_s3("보고서.hwp")
text = Hwarang.extract_text(StringIO.new(data))
```

---

## 시나리오 6: OLE 스트림 탐색 (디버깅/분석)

```ruby
doc = Hwarang::Document.new("보고서.hwp")

# OLE 스트림 목록
doc.streams.each do |name|
  puts name
end
# => "FileHeader"
# => "DocInfo"
# => "BodyText/Section0"
# => "BodyText/Section1"
# => ...

# 문서 정보
puts doc.format       #=> :hwp
puts doc.version      #=> "5.1.0.1"
puts doc.compressed?  #=> true
puts doc.encrypted?   #=> false
puts doc.sections     #=> 3
```

**대응하는 Rust API**: `list_streams(path) -> Result<Vec<String>>`

---

## API 설계 계층

sqlite3-ruby에서 배운 패턴 적용:

```
사용자 코드
    ↓
lib/hwarang/document.rb     # 고수준 Ruby API (편의 메서드, 블록 패턴)
    ↓
ext/hwarang/src/lib.rs      # Rust 바인딩 (magnus, 최소 저수준 API)
    ↓
hwarang 크레이트             # 순수 Rust 텍스트 추출 엔진
```

### Rust에서 노출할 최소 API (private)

| Rust 함수 | Ruby private 메서드 | 역할 |
|---|---|---|
| `extract_text_from_file` | `_extract_text` | 텍스트 추출 핵심 |
| `list_streams` | `_list_streams` | 스트림 목록 |
| format detection | `_detect_format` | 포맷 감지 |

### Ruby에서 구축할 고수준 API (public)

| Ruby 메서드 | 역할 |
|---|---|
| `Hwarang.extract_text(path_or_io)` | 간편 텍스트 추출 |
| `Hwarang.extract_batch(paths)` | 배치 처리 |
| `Hwarang::Document.new(path)` | 문서 객체 생성 |
| `Document#text` | 텍스트 접근 |
| `Document#format` | 포맷 확인 |
| `Document#streams` | 스트림 목록 |

---

## 예외 클래스 매핑

```
HwpError (Rust)                  →  Hwarang::Error (Ruby)
├── Io                           →  Hwarang::IOError
├── InvalidSignature             →  Hwarang::InvalidSignatureError
├── UnsupportedVersion           →  Hwarang::UnsupportedVersionError
├── PasswordProtected            →  Hwarang::PasswordProtectedError
├── StreamNotFound               →  Hwarang::StreamNotFoundError
├── InvalidRecordHeader          →  Hwarang::ParseError
├── DecompressFailed             →  Hwarang::DecompressError
├── DecryptFailed                →  Hwarang::DecryptError
├── Parse                        →  Hwarang::ParseError
├── UnsupportedFormat            →  Hwarang::UnsupportedFormatError
└── Hwpx                         →  Hwarang::HwpxError
```
