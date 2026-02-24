# Hwarang 크레이트 분석

> hwarang은 HWP (한컴 오피스) 문서에서 텍스트를 빠르게 추출하는 Rust 라이브러리.

## 프로젝트 구조

```
hwarang/
├── Cargo.toml
├── src/
│   ├── lib.rs           # 공개 API (extract_text_from_file, list_streams)
│   ├── main.rs          # CLI 바이너리
│   ├── error.rs         # HwpError 에러 타입
│   ├── extract.rs       # 핵심 텍스트 추출 로직 (542줄)
│   ├── hwpx.rs          # HWPX (ZIP+XML) 포맷 지원
│   └── hwp/             # HWP (OLE) 포맷 모듈
│       ├── mod.rs
│       ├── header.rs    # FileHeader 파싱 (버전, 플래그)
│       ├── record.rs    # 레코드 구조 파싱
│       ├── stream.rs    # Raw Deflate 압축 해제
│       ├── crypto.rs    # 배포용 문서 AES-128-ECB 복호화
│       ├── docinfo.rs   # 문서 메타데이터
│       ├── para_text.rs # 문단 텍스트 + 컨트롤 마커 파싱
│       ├── control.rs   # 컨트롤 요소 ID (표, 각주 등)
│       ├── paragraph.rs # (미구현 스텁)
│       └── section.rs   # (미구현 스텁)
└── docs/
```

## 의존성

```toml
cfb = "0.14"         # OLE/CFB (Compound File Binary) 포맷
flate2 = "1"         # Raw Deflate 압축 해제
byteorder = "1"      # 바이트 순서 처리 (LE/BE)
rayon = "1"          # 병렬 처리
clap = "4"           # CLI 인자 파싱
thiserror = "2"      # 에러 매크로
anyhow = "1"         # 에러 컨텍스트
aes = "0.8"          # AES 암호화
ecb = "0.1"          # ECB 모드
zip = "2"            # ZIP 아카이브 (HWPX용)
quick-xml = "0.37"   # XML 파싱 (HWPX/HWPML용)
```

## 공개 API

### 핵심 함수 2개

```rust
/// 파일 경로로부터 텍스트 추출 (포맷 자동 감지)
pub fn extract_text_from_file(path: &Path) -> Result<String>

/// HWP 파일의 OLE 스트림 목록 반환
pub fn list_streams(path: &Path) -> Result<Vec<String>>
```

### 에러 타입

```rust
pub enum HwpError {
    Io(std::io::Error),
    InvalidSignature,           // 잘못된 파일 형식
    UnsupportedVersion(u8, u8, u8, u8),
    PasswordProtected,          // 암호 걸린 문서
    StreamNotFound(String),
    InvalidRecordHeader,
    DecompressFailed(String),
    DecryptFailed(String),
    Parse(String),
    UnsupportedFormat,
    Hwpx(String),
}
```

## 지원 포맷

매직 바이트로 자동 감지:

| 매직 바이트 | 포맷 | 처리 방식 |
|---|---|---|
| `50 4B 03 04` | HWPX | ZIP 열기 → section*.xml 파싱 |
| `D0 CF 11 E0` | HWP | OLE/CFB 컨테이너 → 스트림 파싱 |
| `3C 3F 78 6D` | HWPML | XML 직접 파싱 |

## 텍스트 추출 파이프라인

```
extract_text_from_file(path)
  │
  ├─ HWP (OLE): extract_text_from_hwp()
  │   ├── OLE 컨테이너 열기 (cfb)
  │   ├── /FileHeader 읽기 → 버전, 압축, 암호, 배포 플래그
  │   ├── /DocInfo 읽기 → 섹션 수 파악
  │   ├── 스토리지 결정: "ViewText" (배포) 또는 "BodyText" (일반)
  │   └── 각 섹션:
  │       ├── Section{i} 스트림 읽기
  │       ├── 압축 해제 (Raw Deflate) / 복호화 (AES-128-ECB)
  │       ├── 레코드 파싱 (4바이트 packed header)
  │       └── 재귀적 텍스트 추출 (표→마크다운, 각주, 텍스트박스)
  │
  ├─ HWPX (ZIP+XML): extract_text_from_hwpx()
  │   ├── ZIP 열기
  │   ├── Contents/section*.xml 찾기
  │   └── quick-xml로 <hp:t> 텍스트 추출
  │
  └─ HWPML (XML): extract_text_from_hwpml()
      └── XML 직접 파싱
```

## 복잡한 구조 처리

### 레코드 계층 구조 (HWP)

```
PARA_HEADER (level=0)
  ├─ PARA_TEXT (level=1)        ← UTF-16LE 텍스트 + 컨트롤 마커
  ├─ PARA_CHAR_SHAPE (level=1)
  └─ CTRL_HEADER (level=1)     ← 표, 각주, 텍스트박스 등
      ├─ TABLE (level=2)
      ├─ LIST_HEADER (level=2)  ← 표 셀
      └─ PARA_HEADER (level=2)  ← 셀 안의 중첩 문단
```

### 배포용 문서 복호화

```
1. LCG (Linear Congruential Generator) XOR 디오브퍼스케이션
2. 메타데이터에서 AES 키 추출
3. AES-128-ECB NoPadding 복호화
```

## FFI 현황

**현재: FFI 인터페이스 없음**

- `#[no_mangle]` 함수 없음
- `extern "C"` 선언 없음
- C ABI 호환 레이어 없음

Ruby gem으로 만들 때 magnus 크레이트를 통해 Ruby 바인딩을 추가해야 함.
