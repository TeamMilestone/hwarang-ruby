# sqlite3-ruby 아키텍처 분석

> sqlite3-ruby gem이 Ruby와 C로 작성된 SQLite3 라이브러리를 연결하는 방식을 분석한 문서.
> Rust 검색엔진을 Ruby에서 활용하기 위한 참고 자료로 작성됨.

## 목차

1. [프로젝트 구조 개요](#1-프로젝트-구조-개요)
2. [빌드 시스템](#2-빌드-시스템)
3. [C 확장 초기화 (Init 함수)](#3-c-확장-초기화-init-함수)
4. [TypedData 래핑 패턴](#4-typeddata-래핑-패턴)
5. [Database 클래스 바인딩](#5-database-클래스-바인딩)
6. [Statement 클래스 바인딩](#6-statement-클래스-바인딩)
7. [메모리 관리 및 GC 연동](#7-메모리-관리-및-gc-연동)
8. [데이터 타입 변환](#8-데이터-타입-변환)
9. [콜백 패턴](#9-콜백-패턴)
10. [Ruby 레이어 설계](#10-ruby-레이어-설계)
11. [Rust 적용 시사점](#11-rust-적용-시사점)

---

## 1. 프로젝트 구조 개요

```
sqlite3-ruby/
├── ext/sqlite3/           # C 확장 코드
│   ├── extconf.rb         # 빌드 설정 (mkmf)
│   ├── sqlite3.c          # 진입점: Init_sqlite3_native()
│   ├── sqlite3_ruby.h     # 공통 헤더
│   ├── database.c/h       # SQLite3::Database C 구현
│   ├── statement.c/h      # SQLite3::Statement C 구현
│   ├── backup.c/h         # SQLite3::Backup C 구현
│   ├── aggregator.c/h     # 집계 함수 지원
│   ├── exception.c/h      # 에러 코드 → Ruby 예외 변환
│   └── timespec.h         # 시간 관련 매크로
├── lib/
│   ├── sqlite3.rb         # 진입점: require "sqlite3/sqlite3_native"
│   └── sqlite3/
│       ├── database.rb    # Ruby 측 Database 래퍼 (고수준 API)
│       ├── statement.rb   # Ruby 측 Statement 래퍼
│       ├── resultset.rb   # 결과셋 이터레이터
│       ├── constants.rb   # 상수 정의
│       ├── errors.rb      # 예외 클래스 계층
│       ├── pragmas.rb     # PRAGMA 편의 메서드
│       ├── fork_safety.rb # fork() 안전성
│       └── value.rb       # 값 래퍼
└── sqlite3.gemspec        # 젬 명세
```

**핵심 설계 원칙**: C 확장은 **최소한의 저수준 바인딩**만 제공하고, Ruby 레이어에서 **고수준 API**를 구축한다.

---

## 2. 빌드 시스템

### 2.1 extconf.rb와 mkmf

`ext/sqlite3/extconf.rb`는 Ruby 표준 빌드 도구인 `mkmf`(MakeMakefile)를 사용한다.

```ruby
require "mkmf"
# ...
create_makefile("sqlite3/sqlite3_native")
```

`create_makefile("sqlite3/sqlite3_native")`는:
- `sqlite3_native.so` (Linux), `sqlite3_native.bundle` (macOS), `sqlite3_native.dll` (Windows) 생성
- Ruby에서 `require "sqlite3/sqlite3_native"`으로 로드 가능

### 2.2 SQLite3 라이브러리 링크 방식

두 가지 모드를 지원:

1. **패키지 라이브러리** (기본값): `mini_portile2` gem으로 SQLite3 소스를 다운로드/컴파일하여 정적 링크
2. **시스템 라이브러리**: `--enable-system-libraries` 플래그로 시스템 설치된 SQLite3 사용

### 2.3 기능 탐지

`extconf.rb`에서 `have_func`, `have_type` 등으로 컴파일 타임에 기능을 탐지:

```ruby
have_func("sqlite3_prepare_v2")
have_func("sqlite3_backup_init")
have_type("sqlite3_int64", "sqlite3.h")
```

이 결과는 `extconf.h`에 `#define HAVE_SQLITE3_PREPARE_V2` 형태로 저장되어 조건부 컴파일에 사용됨.

### 2.4 gemspec 연결

```ruby
s.extensions << "ext/sqlite3/extconf.rb"
```

gem 설치 시 자동으로 `extconf.rb`가 실행되어 네이티브 확장이 빌드된다.

---

## 3. C 확장 초기화 (Init 함수)

### 3.1 진입점 규칙

Ruby가 `require "sqlite3/sqlite3_native"` 시 자동으로 `Init_sqlite3_native()` 함수를 호출한다.

**규칙**: `Init_` + 파일명 (경로의 `/`는 `_`로 변환). 이 이름이 일치하지 않으면 로드 실패.

```c
// ext/sqlite3/sqlite3.c
RUBY_FUNC_EXPORTED
void
Init_sqlite3_native(void)
{
    mSqlite3 = rb_define_module("SQLite3");
    cSqlite3Blob = rb_define_class_under(mSqlite3, "Blob", rb_cString);

    sqlite3_initialize();

    init_sqlite3_constants();
    init_sqlite3_database();
    init_sqlite3_statement();
    init_sqlite3_backup();

    rb_define_singleton_method(mSqlite3, "sqlcipher?", using_sqlcipher, 0);
    rb_define_singleton_method(mSqlite3, "libversion", libversion, 0);
    rb_define_singleton_method(mSqlite3, "threadsafe", threadsafe_p, 0);
    rb_define_const(mSqlite3, "SQLITE_VERSION", rb_str_new2(SQLITE_VERSION));
}
```

### 3.2 모듈/클래스 등록 API

| Ruby API 함수 | 역할 |
|---|---|
| `rb_define_module("Name")` | 최상위 모듈 정의 |
| `rb_define_class_under(module, "Name", parent)` | 모듈 하위 클래스 정의 |
| `rb_define_module_under(module, "Name")` | 모듈 하위 모듈 정의 |
| `rb_define_method(klass, "name", func, arity)` | 인스턴스 메서드 정의 |
| `rb_define_singleton_method(obj, "name", func, arity)` | 클래스/싱글턴 메서드 정의 |
| `rb_define_private_method(klass, "name", func, arity)` | private 메서드 정의 |
| `rb_define_alloc_func(klass, func)` | 메모리 할당 함수 등록 |
| `rb_define_const(module, "NAME", value)` | 상수 정의 |

**arity 규칙**:
- 양수 N: 정확히 N개의 인자
- -1: 가변 인자 (C 함수가 `int argc, VALUE *argv` 받음)
- -2: Ruby 배열로 인자를 받음

---

## 4. TypedData 래핑 패턴

### 4.1 C 구조체를 Ruby 객체로 래핑

sqlite3-ruby의 핵심 패턴. C 구조체를 Ruby 객체 안에 캡슐화한다.

#### 1단계: C 구조체 정의

```c
// database.h
struct _sqlite3Ruby {
    sqlite3 *db;           // SQLite3 DB 핸들 (C 라이브러리 포인터)
    VALUE busy_handler;    // Ruby 콜백 객체 (GC가 관리)
    int stmt_timeout;
    struct timespec stmt_deadline;
    rb_pid_t owner;        // fork 안전성을 위한 PID
    int flags;
};
typedef struct _sqlite3Ruby sqlite3Ruby;
```

#### 2단계: rb_data_type_t 정의 (GC 콜백 테이블)

```c
static const rb_data_type_t database_type = {
    .wrap_struct_name = "SQLite3::Backup",
    .function = {
        .dmark = database_mark,    // GC 마킹: Ruby 참조를 추적
        .dfree = deallocate,       // GC 해제: C 리소스 정리
        .dsize = database_memsize, // 메모리 크기 보고
    },
    .flags = RUBY_TYPED_WB_PROTECTED,
};
```

#### 3단계: 할당 함수

```c
static VALUE
allocate(VALUE klass)
{
    sqlite3RubyPtr ctx;
    VALUE object = TypedData_Make_Struct(klass, sqlite3Ruby, &database_type, ctx);
    ctx->owner = getpid();
    return object;
}
```

`TypedData_Make_Struct`는:
1. Ruby 객체를 할당
2. C 구조체 메모리를 할당
3. 구조체 포인터를 ctx에 저장
4. GC 콜백 테이블(database_type)을 연결

#### 4단계: 구조체 추출

```c
sqlite3RubyPtr ctx;
TypedData_Get_Struct(self, sqlite3Ruby, &database_type, ctx);
// 이제 ctx->db로 SQLite3 C 라이브러리 호출 가능
```

---

## 5. Database 클래스 바인딩

### 5.1 메서드 등록

```c
void init_sqlite3_database(void)
{
    cSqlite3Database = rb_define_class_under(mSqlite3, "Database", rb_cObject);

    rb_define_alloc_func(cSqlite3Database, allocate);
    rb_define_private_method(cSqlite3Database, "open_v2", rb_sqlite3_open_v2, 3);
    rb_define_method(cSqlite3Database, "close", sqlite3_rb_close, 0);
    rb_define_method(cSqlite3Database, "last_insert_row_id", last_insert_row_id, 0);
    // ... 등등
}
```

### 5.2 C와 Ruby의 역할 분담

**C에서 구현** (저수준, 성능 중요):
- `open_v2`, `close` - DB 열기/닫기
- `step` - SQL 실행 한 행 가져오기
- `bind_param` - 파라미터 바인딩
- `define_function` - 사용자 정의 함수
- `busy_handler` - 콜백 등록

**Ruby에서 구현** (고수준, 편의성):
- `initialize` - 옵션 처리, 모드 설정
- `execute`, `query` - 고수준 쿼리 API
- `transaction`, `commit`, `rollback`
- `create_function`, `create_aggregate` - FunctionProxy 래핑
- `prepare` - Statement 생성 + 블록 패턴

### 5.3 패턴: private C 메서드 + public Ruby 메서드

```c
// C: private 메서드로 등록
rb_define_private_method(cSqlite3Database, "open_v2", rb_sqlite3_open_v2, 3);
```

```ruby
# Ruby: public initialize에서 private C 메서드 호출
def initialize(file, options = {}, zvfs = nil)
  open_v2(file.encode("utf-8"), mode, zvfs)
end
```

이 패턴으로 C는 최소한의 기능만 노출하고, Ruby가 검증/변환/편의 로직을 담당.

---

## 6. Statement 클래스 바인딩

### 6.1 구조체

```c
struct _sqlite3StmtRuby {
    sqlite3_stmt *st;   // SQLite prepared statement 핸들
    sqlite3Ruby *db;    // 부모 Database의 C 구조체 포인터 (직접 참조, 빠른 접근)
    int done_p;         // 실행 완료 플래그
};
```

### 6.2 핵심 흐름

```
Ruby: Statement.new(db, sql)
  └── C: prepare(self, db, sql)
        ├── sqlite3_database_unwrap(db) → db_ctx
        ├── sqlite3_prepare_v2(db_ctx->db, sql, ..., &ctx->st, &tail)
        └── return tail (남은 SQL)

Ruby: stmt.step
  └── C: step(self)
        ├── sqlite3_step(stmt)
        ├── sqlite3_column_type으로 타입 확인
        ├── 각 컬럼을 Ruby VALUE로 변환
        └── return frozen Array (행 데이터)
```

---

## 7. 메모리 관리 및 GC 연동

### 7.1 GC 콜백 3종

| 콜백 | 역할 | Database 예시 |
|---|---|---|
| `dmark` | Ruby 참조 마킹 (GC가 회수하지 않도록) | `rb_gc_mark(c->busy_handler)` |
| `dfree` | C 리소스 해제 | `sqlite3_close_v2(ctx->db); xfree(ctx)` |
| `dsize` | 메모리 크기 보고 (GC 휴리스틱용) | `return sizeof(*c)` |

### 7.2 마킹이 필요한 경우

C 구조체가 Ruby 객체(VALUE)를 참조하는 경우 반드시 `dmark`에서 마킹해야 한다:

```c
// Database: busy_handler (Ruby Proc)를 C 구조체에 저장
static void database_mark(void *ctx) {
    sqlite3RubyPtr c = (sqlite3RubyPtr)ctx;
    rb_gc_mark(c->busy_handler);  // GC에게 이 객체가 아직 사용 중임을 알림
}
```

### 7.3 Write Barrier

```c
RB_OBJ_WRITE(self, &ctx->busy_handler, block);
```

Ruby의 세대별 GC를 위해, C 구조체에 Ruby 객체 참조를 저장할 때 `RB_OBJ_WRITE` 사용.

### 7.4 Statement의 GC

Statement는 마킹이 필요 없음 — Ruby 참조를 C 구조체에 저장하지 않기 때문:

```c
static const rb_data_type_t statement_type = {
    "SQLite3::Backup",
    {
        NULL,                    // dmark: 없음
        statement_deallocate,    // dfree: sqlite3_finalize(s->st)
        statement_memsize,
    },
    0, 0,
    RUBY_TYPED_FREE_IMMEDIATELY | RUBY_TYPED_WB_PROTECTED,
};
```

### 7.5 GC 안전한 참조 보존

콜백 함수나 collation을 등록할 때, Ruby 객체가 GC되지 않도록 인스턴스 변수에 저장:

```c
// C에서 SQLite에 콜백 등록 후, Ruby 측에 참조 저장
rb_hash_aset(rb_iv_get(self, "@functions"), name, block);
rb_hash_aset(rb_iv_get(self, "@collations"), name, comparator);
```

---

## 8. 데이터 타입 변환

### 8.1 SQLite → Ruby (읽기)

| SQLite 타입 | Ruby 타입 | 변환 함수 |
|---|---|---|
| `SQLITE_INTEGER` | `Integer` (Fixnum/Bignum) | `LL2NUM(sqlite3_column_int64())` |
| `SQLITE_FLOAT` | `Float` | `rb_float_new(sqlite3_column_double())` |
| `SQLITE_TEXT` | `String` (UTF-8, frozen) | `rb_utf8_str_new()` |
| `SQLITE_BLOB` | `String` (ASCII-8BIT, frozen) | `rb_str_new()` |
| `SQLITE_NULL` | `nil` | `Qnil` |

### 8.2 Ruby → SQLite (바인딩)

| Ruby 타입 | SQLite 함수 | 비고 |
|---|---|---|
| `T_STRING` + Blob/ASCII-8BIT | `sqlite3_bind_blob()` | 바이너리 데이터 |
| `T_STRING` + UTF-16 | `sqlite3_bind_text16()` | UTF-16 문자열 |
| `T_STRING` (기타) | `sqlite3_bind_text()` | UTF-8로 변환 후 바인딩 |
| `T_FIXNUM` | `sqlite3_bind_int64()` | 64비트 정수 |
| `T_BIGNUM` | `sqlite3_bind_int64()` | bignum_to_int64로 변환 시도, 실패 시 Float |
| `T_FLOAT` | `sqlite3_bind_double()` | 부동소수점 |
| `T_NIL` | `sqlite3_bind_null()` | NULL |

### 8.3 주요 변환 매크로/함수

```c
// Ruby → C
NUM2INT(value)       // VALUE → int
NUM2DBL(value)       // VALUE → double
StringValuePtr(str)  // VALUE → char*
RSTRING_LEN(str)     // String 길이
FIX2LONG(value)      // Fixnum → long

// C → Ruby
INT2NUM(n)           // int → VALUE
INT2FIX(n)           // 작은 정수 → VALUE (할당 없음)
LL2NUM(n)            // int64 → VALUE
rb_float_new(d)      // double → VALUE
rb_str_new(ptr, len) // char* → String
rb_str_new2(cstr)    // null-terminated char* → String
Qnil, Qtrue, Qfalse  // nil, true, false
```

---

## 9. 콜백 패턴

### 9.1 기본 패턴: C 함수 포인터 → Ruby 블록 호출

SQLite3의 콜백 시스템을 Ruby의 블록/Proc으로 연결:

```
Ruby:  db.busy_handler { |count| ... }
  │
  ├── Ruby Proc을 C 구조체에 저장: ctx->busy_handler = block
  ├── SQLite에 C 콜백 등록: sqlite3_busy_handler(db, rb_sqlite3_busy_handler, ctx)
  │
  └── SQLite가 콜백 호출 시:
      rb_sqlite3_busy_handler(context, count)
        ├── ctx = (sqlite3RubyPtr)context
        ├── handle = ctx->busy_handler  (저장해둔 Ruby Proc)
        └── rb_funcall(handle, rb_intern("call"), 1, INT2NUM(count))
```

### 9.2 사용자 정의 함수

```c
// Ruby 블록을 sqlite3_user_data로 전달
sqlite3_create_function(ctx->db, name, arity, flags,
    (void *)block,      // user_data로 Ruby Proc 저장
    rb_sqlite3_func,    // C 래퍼 함수
    NULL, NULL);

// C 래퍼에서 Ruby 호출
static void rb_sqlite3_func(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
    VALUE callable = (VALUE)sqlite3_user_data(ctx);  // 저장된 Proc 꺼냄
    // argv를 Ruby 배열로 변환
    VALUE result = rb_apply(callable, rb_intern("call"), params);
    set_sqlite3_func_result(ctx, result);  // 결과를 다시 SQLite 타입으로 변환
}
```

### 9.3 집계 함수 (Aggregator)

가장 복잡한 콜백 패턴:

1. `AggregatorWrapper` - 팩토리 클래스를 감쌈 (한 번 등록)
2. `AggregatorInstance` - 실행 중인 집계 인스턴스 (쿼리당 생성)
3. `sqlite3_aggregate_context`로 실행 컨텍스트별 상태 관리
4. `rb_protect`로 Ruby 예외를 안전하게 처리

### 9.4 GC 보호 전략

콜백에 사용되는 Ruby 객체가 GC되면 안 되므로:
- 인스턴스 변수에 저장: `@functions`, `@collations`, `-aggregators`
- `rb_gc_register_mark_object`로 전역 보호
- C 구조체의 `dmark` 콜백으로 마킹

---

## 10. Ruby 레이어 설계

### 10.1 계층 구조

```
사용자 코드
    ↓
lib/sqlite3/database.rb   (고수준 Ruby API: execute, query, transaction ...)
    ↓
ext/sqlite3/database.c    (저수준 C 바인딩: open_v2, step, bind_param ...)
    ↓
libsqlite3                (SQLite3 C 라이브러리)
```

### 10.2 Ruby 레이어의 역할

1. **옵션 처리 및 검증**: `initialize`에서 mode 플래그 조합, 인코딩 처리
2. **편의 API**: `execute(sql, params)` → prepare → bind → step → collect
3. **리소스 관리**: `prepare { |stmt| ... }` 블록 패턴으로 자동 close
4. **이터레이터**: `Statement`가 `Enumerable`을 include, `each` 메서드 제공
5. **호환성 래퍼**: `create_aggregate`, `create_aggregate_handler` 등 다양한 API 스타일
6. **Fork 안전성**: `ForkSafety` 모듈이 `Process._fork`를 hooking하여 DB 연결 관리

### 10.3 Require 체인

```ruby
# lib/sqlite3.rb
require "sqlite3/sqlite3_native"  # C 확장 로드 (Init_sqlite3_native 호출)
require "sqlite3/database"         # Ruby 래퍼 로드
require "sqlite3/version"
```

---

## 11. Rust 적용 시사점

### 11.1 동일한 방식 적용 가능

sqlite3-ruby와 동일한 패턴으로 Rust 검색엔진을 Ruby에 연결할 수 있다:

**방법 A: C 확장 직접 작성 + Rust FFI**
```
Ruby → C 확장 (Init_ 함수) → Rust 라이브러리 (extern "C" FFI)
```

**방법 B: rb-sys/magnus 크레이트 사용 (권장)**
```
Ruby → Rust 확장 (magnus가 Init_ 함수 자동 생성) → Rust 검색엔진
```

### 11.2 sqlite3-ruby에서 배운 핵심 패턴

| 패턴 | sqlite3-ruby (C) | Rust 적용 |
|---|---|---|
| 진입점 | `Init_sqlite3_native()` | `magnus::init!` 매크로 |
| 구조체 래핑 | `TypedData_Make_Struct` | `magnus::TypedData` trait |
| GC 마킹 | `dmark` 콜백 | `magnus::DataTypeFunctions::mark` |
| GC 해제 | `dfree` 콜백 | `Drop` trait 구현 |
| 메서드 정의 | `rb_define_method` | `#[magnus::method]` 어트리뷰트 |
| 타입 변환 | `NUM2INT`, `rb_str_new` 등 | `magnus::TryConvert`, `Into<Value>` |
| 예외 | `rb_raise` | `magnus::Error` 반환 |
| 콜백 | `rb_funcall` | `magnus::block::Proc` |

### 11.3 권장 아키텍처

```
ru-by-st/
├── ext/
│   └── ru_by_st/
│       ├── extconf.rb          # Rust 빌드 설정 (rb-sys)
│       └── src/
│           ├── lib.rs          # Init 함수 + 모듈 등록
│           ├── search_engine.rs # SearchEngine 클래스 (TypedData)
│           └── result.rs       # 검색 결과 변환
├── lib/
│   ├── ru_by_st.rb            # require "ru_by_st/ru_by_st"
│   └── ru_by_st/
│       ├── search_engine.rb   # 고수준 Ruby API
│       └── result.rb          # Ruby 결과 래퍼
└── Cargo.toml                 # Rust 의존성
```

### 11.4 핵심 체크리스트

sqlite3-ruby 분석에서 도출한 Rust 확장 구현 시 체크리스트:

- [ ] `extconf.rb`에서 `create_makefile` 호출 (또는 rb-sys 사용)
- [ ] `Init_` 함수에서 모듈/클래스 등록
- [ ] Rust 구조체를 TypedData로 래핑 (검색 인덱스 핸들)
- [ ] Drop trait으로 C 리소스 정리 (Rust는 자동)
- [ ] Ruby VALUE 참조 시 GC 마킹 구현
- [ ] 검색 결과를 Ruby Array/Hash로 변환
- [ ] 에러를 Ruby 예외로 변환
- [ ] Ruby 레이어에서 고수준 API 구축
- [ ] fork 안전성 고려 (필요 시)
