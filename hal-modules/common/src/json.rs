use serde::de::DeserializeOwned;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::HapError;

pub fn parse_params<T: DeserializeOwned>(params_json: *const c_char) -> Result<T, HapError> {
    if params_json.is_null() {
        return Err(HapError::invalid_param("argument is null"));
    }
    let s = unsafe { CStr::from_ptr(params_json) }
        .to_str()
        .map_err(|_| HapError::invalid_param("argument is not valid UTF-8"))?;
    serde_json::from_str(s).map_err(|e| HapError::invalid_param(format!("JSON parse failed: {e}")))
}

pub fn to_json_cstring<T: serde::Serialize>(val: &T) -> Result<CString, HapError> {
    let json = serde_json::to_string(val)?;
    CString::new(json).map_err(|_| HapError::internal("return value contains NUL byte"))
}

pub fn ok_json<T: serde::Serialize>(val: &T) -> *const c_char {
    match to_json_cstring(val) {
        Ok(cs) => cs.into_raw(),
        Err(e) => error_json(&e),
    }
}

pub fn error_json(e: &HapError) -> *const c_char {
    let err = serde_json::json!({ "error": { "code": e.code, "message": e.message } });
    match CString::new(err.to_string()) {
        Ok(cs) => cs.into_raw(),
        Err(_) => std::ptr::null(),
    }
}
