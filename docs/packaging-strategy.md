# 패키징 전략: hwarang gem에 Rust 본체를 포함하는 방법

## sqlite3 gem은 어떻게 하고 있나?

sqlite3 gem은 **3가지 모드**를 지원:

### 모드 1: 소스 포함 + 설치 시 컴파일 (기본값)

```
gem install sqlite3
  → mini_portile2가 sqlite3 C 소스를 다운로드
  → 사용자 머신에서 컴파일
  → 정적 링크 (.a)
  → C 확장과 합쳐서 sqlite3_native.so 생성
```

- **brew 불필요**, 인터넷만 있으면 됨
- 단점: 컴파일 시간 소요, C 컴파일러 필요

### 모드 2: 프리컴파일된 바이너리 (현대 gem들의 추세)

```
gem install sqlite3 --platform x86_64-linux
  → 이미 컴파일된 .so 파일이 gem에 포함
  → 컴파일 불필요, 즉시 사용
```

- 플랫폼별 gem 배포: `sqlite3-2.x.x-x86_64-linux.gem`, `sqlite3-2.x.x-arm64-darwin.gem` 등
- `rake-compiler-dock`으로 크로스 컴파일

### 모드 3: 시스템 라이브러리 사용

```
brew install sqlite3
gem install sqlite3 -- --enable-system-libraries
  → 시스템에 설치된 libsqlite3 사용
  → C 확장만 컴파일
```

---

## hwarang gem 패키징 방식

hwarang은 **순수 Rust 라이브러리**이므로 sqlite3보다 오히려 단순합니다.
외부 C 라이브러리(sqlite3 같은)에 의존하지 않고, 모든 것이 Rust 크레이트로 해결됩니다.

### 권장: 소스 포함 + 설치 시 컴파일

```
hwarang gem 구조:
├── ext/hwarang/
│   ├── extconf.rb          # rb-sys 사용
│   ├── Cargo.toml          # Rust 의존성
│   └── src/
│       └── lib.rs          # magnus 바인딩 코드
├── lib/
│   ├── hwarang.rb
│   └── hwarang/
│       └── document.rb
└── hwarang.gemspec
```

```ruby
# ext/hwarang/extconf.rb
require "mkmf"
require "rb_sys/mkmf"

create_rust_makefile("hwarang/hwarang") do |r|
  r.auto_install = true  # Rust 툴체인 없으면 자동 설치
end
```

```ruby
# hwarang.gemspec
Gem::Specification.new do |s|
  s.name = "hwarang"
  s.extensions = ["ext/hwarang/extconf.rb"]
  # Rust 소스가 gem에 포함됨
  # gem install 시 Rust 컴파일러가 자동으로 빌드
end
```

### hwarang 크레이트를 gem에 포함하는 방법

**방법 A: 소스 직접 포함**

```
ext/hwarang/
├── Cargo.toml
└── src/
    └── lib.rs    # magnus 바인딩 + hwarang 소스를 path로 참조
```

```toml
# ext/hwarang/Cargo.toml
[dependencies]
magnus = "0.8"
hwarang = { path = "../../../hwarang" }  # 개발 시
# hwarang = "0.1"                        # 배포 시 crates.io에서
```

**방법 B: hwarang 소스 전체를 gem에 복사**

```
ext/hwarang/
├── Cargo.toml
├── src/
│   └── lib.rs           # magnus 바인딩
└── hwarang-core/        # hwarang 크레이트 소스 복사본
    ├── Cargo.toml
    └── src/
        └── ...
```

**방법 C: crates.io에 hwarang 배포 후 의존**

```toml
# ext/hwarang/Cargo.toml
[dependencies]
magnus = "0.8"
hwarang = "0.1"  # crates.io에서 자동 다운로드
```

### 프리컴파일 배포 (선택)

`rb-sys`와 `rake-compiler`를 사용하면 플랫폼별 프리컴파일 gem 생성 가능:

```bash
# GitHub Actions에서 크로스 컴파일
rake native:x86_64-linux
rake native:arm64-darwin
rake native:x86_64-darwin
rake native:x64-mingw-ucrt
```

사용자 입장에서는:
```bash
gem install hwarang
# → 자신의 플랫폼에 맞는 프리컴파일 gem이 자동 선택
# → Rust 컴파일러 불필요!
```

---

## 비교표

| 항목 | sqlite3 gem | hwarang gem (계획) |
|---|---|---|
| 네이티브 언어 | C | Rust |
| 외부 라이브러리 | libsqlite3 | 없음 (순수 Rust) |
| 빌드 도구 | mkmf + mini_portile2 | rb-sys + Cargo |
| 컴파일러 필요 | C 컴파일러 (gcc/clang) | Rust 컴파일러 (rustc) |
| brew 설치 필요? | 아니오 (기본값) | 아니오 |
| 프리컴파일 지원? | 예 | 예 (rb-sys 지원) |
| 소스 포함? | SQLite3 소스 다운로드 | hwarang Rust 소스 포함 |

---

## 결론

**hwarang은 컴파일된 결과물 형태가 아니라, Rust 소스코드가 gem에 포함됩니다.**

`gem install hwarang` 실행 시:
1. Rust 소스코드 + Cargo.toml이 포함된 gem 다운로드
2. `extconf.rb`가 `cargo build --release` 실행
3. 컴파일된 `.so`/`.bundle`/`.dll` 생성
4. Ruby에서 `require "hwarang/hwarang"` 으로 로드

brew나 별도 설치 불필요. sqlite3 gem과 동일한 방식.
프리컴파일 gem을 배포하면 Rust 컴파일러조차 불필요.
