# Ruby C Extension API 핵심 레퍼런스

> sqlite3-ruby 분석에서 발견된 Ruby C API 패턴 정리.
> Rust(magnus/rb-sys)에서도 동일한 개념이 적용됨.

## VALUE 타입

Ruby의 모든 객체는 C에서 `VALUE` 타입으로 표현된다. `VALUE`는 포인터 크기의 정수.

### 특수 VALUE

```c
Qnil    // nil    (0x04 on 64-bit)
Qtrue   // true   (0x02)
Qfalse  // false  (0x00)
Qundef  // 미정의
```

### VALUE 검사

```c
NIL_P(value)    // value == Qnil?
RTEST(value)    // value != Qnil && value != Qfalse?
TYPE(value)     // T_FIXNUM, T_STRING, T_FLOAT, T_NIL, T_ARRAY, T_HASH ...
CLASS_OF(value) // 클래스 VALUE 반환
```

---

## 모듈/클래스 정의

```c
// 모듈
VALUE mMyModule = rb_define_module("MyModule");
VALUE mSub = rb_define_module_under(mMyModule, "SubModule");

// 클래스
VALUE cMyClass = rb_define_class_under(mMyModule, "MyClass", rb_cObject);

// 상수
rb_define_const(mMyModule, "VERSION", rb_str_new2("1.0.0"));
```

---

## 메서드 정의

```c
// 인스턴스 메서드: def method_name(arg1, arg2)
rb_define_method(cMyClass, "method_name", c_function, 2);

// 가변 인자: def method_name(*args)
rb_define_method(cMyClass, "method_name", c_function, -1);
// C 시그니처: VALUE c_function(int argc, VALUE *argv, VALUE self)

// 클래스 메서드: def self.class_method
rb_define_singleton_method(cMyClass, "class_method", c_function, 0);

// private 메서드
rb_define_private_method(cMyClass, "internal", c_function, 1);

// 할당 함수 (new 호출 시 메모리 할당)
rb_define_alloc_func(cMyClass, allocate_function);
```

---

## TypedData 래핑 (핵심 패턴)

### 정의

```c
// 1. C 구조체
typedef struct {
    void *internal_handle;
    VALUE ruby_reference;
} MyStruct;

// 2. GC 콜백
static void my_mark(void *ptr) {
    MyStruct *s = (MyStruct *)ptr;
    rb_gc_mark(s->ruby_reference);  // Ruby 참조 보호
}

static void my_free(void *ptr) {
    MyStruct *s = (MyStruct *)ptr;
    // 외부 리소스 해제
    if (s->internal_handle) {
        close_handle(s->internal_handle);
    }
    xfree(s);  // 구조체 자체 해제
}

static size_t my_memsize(const void *ptr) {
    return sizeof(MyStruct);
}

// 3. 타입 정보
static const rb_data_type_t my_type = {
    .wrap_struct_name = "MyModule::MyClass",
    .function = {
        .dmark = my_mark,
        .dfree = my_free,
        .dsize = my_memsize,
    },
    .flags = RUBY_TYPED_FREE_IMMEDIATELY | RUBY_TYPED_WB_PROTECTED,
};
```

### 사용

```c
// 할당
static VALUE allocate(VALUE klass) {
    MyStruct *ctx;
    VALUE obj = TypedData_Make_Struct(klass, MyStruct, &my_type, ctx);
    ctx->internal_handle = NULL;
    ctx->ruby_reference = Qnil;
    return obj;
}

// 추출
MyStruct *ctx;
TypedData_Get_Struct(self, MyStruct, &my_type, ctx);
```

---

## 타입 변환

### C → Ruby

```c
INT2NUM(42)              // int → Integer
INT2FIX(42)              // 작은 int → Integer (할당 없음, 빠름)
LL2NUM(int64_value)      // int64_t → Integer
SIZET2NUM(size_t_value)  // size_t → Integer
rb_float_new(3.14)       // double → Float
rb_str_new(ptr, len)     // bytes → String
rb_str_new2(cstr)        // null-terminated → String
rb_str_new_cstr(cstr)    // 동일
rb_utf8_str_new(ptr,len) // UTF-8 String
rb_utf8_str_new_cstr(s)  // UTF-8 String (null-terminated)
ID2SYM(rb_intern("name"))// → :name Symbol
```

### Ruby → C

```c
NUM2INT(value)          // Integer → int
NUM2LONG(value)         // Integer → long
FIX2LONG(value)         // Fixnum → long (빠름, 검증 없음)
NUM2DBL(value)          // Numeric → double
StringValuePtr(value)   // String → char* (null 보장 안됨)
StringValueCStr(value)  // String → char* (null-terminated 보장)
RSTRING_PTR(value)      // String 내부 포인터
RSTRING_LEN(value)      // String 길이
rb_intern("method")     // 문자열 → ID (Symbol 내부 표현)
```

---

## Ruby 메서드 호출 (C에서)

```c
// 기본 호출
rb_funcall(receiver, rb_intern("method_name"), 2, arg1, arg2);

// 배열로 인자 전달
rb_apply(receiver, rb_intern("call"), args_array);

// 안전한 호출 (예외 잡기)
int state;
VALUE result = rb_protect(protected_func, arg, &state);
if (state) { /* 예외 발생 */ }
```

---

## 배열/해시 조작

```c
// Array
VALUE ary = rb_ary_new2(capacity);  // 용량 지정 생성
rb_ary_push(ary, value);            // 끝에 추가
rb_ary_store(ary, index, value);    // 인덱스에 저장

// Hash
VALUE hash = rb_hash_new();
rb_hash_aset(hash, key, value);     // hash[key] = value
VALUE val = rb_hash_aref(hash, key);// hash[key]
```

---

## 예외 처리

```c
// 예외 발생
rb_raise(rb_eRuntimeError, "error message: %s", detail);
rb_raise(rb_eArgError, "wrong number of arguments");
rb_raise(rb_eTypeError, "expected String, got %s", rb_class2name(CLASS_OF(val)));

// 커스텀 예외 클래스 사용
VALUE klass = rb_path2class("SQLite3::Exception");
VALUE exception = rb_exc_new2(klass, "error message");
rb_iv_set(exception, "@code", INT2FIX(error_code));
rb_exc_raise(exception);
```

---

## 인스턴스 변수

```c
rb_iv_set(self, "@name", value);       // 인스턴스 변수 설정
VALUE val = rb_iv_get(self, "@name");  // 인스턴스 변수 읽기
```

---

## 블록/Proc

```c
// 블록이 주어졌는지 확인
rb_block_given_p()

// 블록을 Proc으로 변환
VALUE block = rb_block_proc();

// Proc 호출
rb_funcall(proc, rb_intern("call"), 1, arg);

// 인자 파싱 (가변 인자 메서드에서)
VALUE arg1, arg2;
rb_scan_args(argc, argv, "11", &arg1, &arg2);  // 1필수, 1선택
```

---

## Write Barrier (세대별 GC)

```c
// C 구조체에 Ruby VALUE를 저장할 때
RB_OBJ_WRITE(wrapper_object, &struct->ruby_field, new_value);

// 전역 변수에 Ruby 객체 보호
rb_gc_register_mark_object(value);
```

---

## 인코딩

```c
rb_utf8_encindex()           // UTF-8 인코딩 인덱스
rb_ascii8bit_encindex()      // ASCII-8BIT (바이너리)
rb_enc_get_index(str)        // 문자열의 인코딩 인덱스
rb_enc_associate_index(str, idx)  // 인코딩 설정
rb_str_export_to_enc(str, enc)    // 인코딩 변환
rb_str_encode(str, encoding, 0, Qnil)  // String#encode
```

---

## freeze

```c
rb_obj_freeze(value);  // 객체 동결 (변경 불가)
// sqlite3-ruby는 반환하는 문자열과 배열을 freeze하여 성능과 안전성 확보
```
