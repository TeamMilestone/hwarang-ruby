use std::path::Path;

use magnus::class::Class;
use magnus::value::{InnerRef, Lazy, ReprValue};
use magnus::{function, prelude::*, Error, ExceptionClass, RHash, Ruby};
use rayon::prelude::*;

macro_rules! define_lazy_error {
    ($static_name:ident, $ruby_name:expr, $parent:expr) => {
        static $static_name: Lazy<ExceptionClass> = Lazy::new(|ruby| {
            let parent = $parent.get_inner_ref_with(&ruby);
            let module = ruby.define_module("Hwarang").unwrap();
            let cls = module
                .define_class($ruby_name, parent.as_r_class())
                .unwrap();
            ExceptionClass::from_value(cls.as_value()).unwrap()
        });
    };
}

static HWARANG_ERROR: Lazy<ExceptionClass> = Lazy::new(|ruby| {
    let module = ruby.define_module("Hwarang").unwrap();
    let cls = module
        .define_class("Error", ruby.exception_standard_error().as_r_class())
        .unwrap();
    ExceptionClass::from_value(cls.as_value()).unwrap()
});

define_lazy_error!(FILE_ERROR, "FileError", HWARANG_ERROR);
define_lazy_error!(
    INVALID_SIGNATURE_ERROR,
    "InvalidSignatureError",
    HWARANG_ERROR
);
define_lazy_error!(
    UNSUPPORTED_VERSION_ERROR,
    "UnsupportedVersionError",
    HWARANG_ERROR
);
define_lazy_error!(
    PASSWORD_PROTECTED_ERROR,
    "PasswordProtectedError",
    HWARANG_ERROR
);
define_lazy_error!(
    STREAM_NOT_FOUND_ERROR,
    "StreamNotFoundError",
    HWARANG_ERROR
);
define_lazy_error!(
    INVALID_RECORD_HEADER_ERROR,
    "InvalidRecordHeaderError",
    HWARANG_ERROR
);
define_lazy_error!(
    DECOMPRESS_FAILED_ERROR,
    "DecompressFailedError",
    HWARANG_ERROR
);
define_lazy_error!(DECRYPT_FAILED_ERROR, "DecryptFailedError", HWARANG_ERROR);
define_lazy_error!(PARSE_ERROR, "ParseError", HWARANG_ERROR);
define_lazy_error!(
    UNSUPPORTED_FORMAT_ERROR,
    "UnsupportedFormatError",
    HWARANG_ERROR
);
define_lazy_error!(HWPX_ERROR, "HwpxError", HWARANG_ERROR);

fn hwp_error_to_magnus(ruby: &Ruby, err: hwarang_core::error::HwpError) -> Error {
    use hwarang_core::error::HwpError;
    let msg = err.to_string();
    let cls = match &err {
        HwpError::Io(_) => *FILE_ERROR.get_inner_ref_with(ruby),
        HwpError::InvalidSignature => *INVALID_SIGNATURE_ERROR.get_inner_ref_with(ruby),
        HwpError::UnsupportedVersion(..) => *UNSUPPORTED_VERSION_ERROR.get_inner_ref_with(ruby),
        HwpError::PasswordProtected => *PASSWORD_PROTECTED_ERROR.get_inner_ref_with(ruby),
        HwpError::StreamNotFound(_) => *STREAM_NOT_FOUND_ERROR.get_inner_ref_with(ruby),
        HwpError::InvalidRecordHeader => *INVALID_RECORD_HEADER_ERROR.get_inner_ref_with(ruby),
        HwpError::DecompressFailed(_) => *DECOMPRESS_FAILED_ERROR.get_inner_ref_with(ruby),
        HwpError::DecryptFailed(_) => *DECRYPT_FAILED_ERROR.get_inner_ref_with(ruby),
        HwpError::Parse(_) => *PARSE_ERROR.get_inner_ref_with(ruby),
        HwpError::UnsupportedFormat => *UNSUPPORTED_FORMAT_ERROR.get_inner_ref_with(ruby),
        HwpError::Hwpx(_) => *HWPX_ERROR.get_inner_ref_with(ruby),
    };
    Error::new(cls, msg)
}

fn extract_text(ruby: &Ruby, path: String) -> Result<String, Error> {
    hwarang_core::extract_text_from_file(Path::new(&path)).map_err(|e| hwp_error_to_magnus(ruby, e))
}

fn list_streams(ruby: &Ruby, path: String) -> Result<Vec<String>, Error> {
    hwarang_core::list_streams(Path::new(&path)).map_err(|e| hwp_error_to_magnus(ruby, e))
}

fn extract_batch(ruby: &Ruby, paths: Vec<String>) -> Result<RHash, Error> {
    let results: Vec<(String, Result<String, String>)> = paths
        .par_iter()
        .map(|p| {
            let result = hwarang_core::extract_text_from_file(Path::new(p));
            match result {
                Ok(text) => (p.clone(), Ok(text)),
                Err(e) => (p.clone(), Err(e.to_string())),
            }
        })
        .collect();

    let hash = ruby.hash_new();
    for (path, result) in results {
        let inner = ruby.hash_new();
        match result {
            Ok(text) => {
                inner.aset(ruby.str_new("text"), ruby.str_new(&text))?;
            }
            Err(msg) => {
                inner.aset(ruby.str_new("error"), ruby.str_new(&msg))?;
            }
        }
        hash.aset(ruby.str_new(&path), inner)?;
    }
    Ok(hash)
}

#[magnus::init(name = "hwarang")]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("Hwarang")?;

    // Force-initialize all error classes
    Lazy::force(&HWARANG_ERROR, ruby);
    Lazy::force(&FILE_ERROR, ruby);
    Lazy::force(&INVALID_SIGNATURE_ERROR, ruby);
    Lazy::force(&UNSUPPORTED_VERSION_ERROR, ruby);
    Lazy::force(&PASSWORD_PROTECTED_ERROR, ruby);
    Lazy::force(&STREAM_NOT_FOUND_ERROR, ruby);
    Lazy::force(&INVALID_RECORD_HEADER_ERROR, ruby);
    Lazy::force(&DECOMPRESS_FAILED_ERROR, ruby);
    Lazy::force(&DECRYPT_FAILED_ERROR, ruby);
    Lazy::force(&PARSE_ERROR, ruby);
    Lazy::force(&UNSUPPORTED_FORMAT_ERROR, ruby);
    Lazy::force(&HWPX_ERROR, ruby);

    module.define_module_function("extract_text", function!(extract_text, 1))?;
    module.define_module_function("list_streams", function!(list_streams, 1))?;
    module.define_module_function("extract_batch", function!(extract_batch, 1))?;

    Ok(())
}
