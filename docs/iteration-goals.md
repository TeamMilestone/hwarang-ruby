# Iteration Goals

## Iteration 1: Hwarang Ruby Gem (2026-02-24)

### 목표
hwarang Rust 크레이트를 Ruby gem으로 만들어 49,353개 HWP/HWPX 파일을 처리.

### 구현 사항
1. **hwarang 크레이트 CLI 분리**: `clap`, `anyhow`, `rayon`을 `cli` feature 뒤로 이동
2. **Ruby gem 구조**: magnus 0.8 + rb-sys 패턴
3. **API 3개**: `Hwarang.extract_text`, `Hwarang.list_streams`, `Hwarang.extract_batch`
4. **에러 매핑**: HwpError 11개 variant → Hwarang::Error 하위 11개 예외 클래스
5. **병렬 처리**: `extract_batch`에서 rayon `par_iter` 사용

### 결과
- 단위 테스트: **9/9 통과**
- 배치 테스트: **49,321/49,353 성공** (32건 실패, 예상 범위 내)
  - 31건: Unsupported file format
  - 1건: HWPX XML 파싱 에러
- 처리 시간: **44.51초** (1,109 files/s)
  - Rust CLI 대비 동등 이상 성능 (CLI 48초)

### 주요 기술 결정
- `magnus::init(name = "hwarang")`: 패키지명 `hwarang-ruby`와 Init 함수명 `Init_hwarang` 불일치 해결
- `hwarang_core = { package = "hwarang" }`: workspace 내 패키지명 충돌 방지
- `ExceptionClass::from_value(cls.as_value())`: magnus에서 RClass→ExceptionClass 변환 패턴
