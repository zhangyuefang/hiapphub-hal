use std::ffi::CString;
use std::os::raw::c_char;

/// 将 Rust 字符串转为 C 字符串指针（调用方需通过 hap_free_string 释放）
pub fn str_to_c(s: &str) -> *const c_char {
    CString::new(s).map(|cs| cs.into_raw() as *const c_char).unwrap_or(std::ptr::null())
}

/// 释放由 CString::into_raw 产生的指针
///
/// # Safety
/// ptr 必须是 CString::into_raw 返回的指针
pub unsafe fn free_c_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        let _ = unsafe { CString::from_raw(ptr) };
    }
}

/// 生成模块的标准 hap_free_string 导出
#[macro_export]
macro_rules! hap_free_string {
    () => {
        #[no_mangle]
        pub unsafe extern "C" fn hap_free_string(ptr: *mut std::os::raw::c_char) {
            hap_common::ffi::free_c_string(ptr);
        }
    };
}

/// 生成模块的 hap_module_init 导出
#[macro_export]
macro_rules! hap_module_init {
    ($name:expr) => {
        #[no_mangle]
        pub extern "C" fn hap_module_init(
            ctx: *const hap_common::context::HapContext,
        ) -> *const std::os::raw::c_char {
            hap_common::context::store_context(ctx);
            hap_common::ffi::str_to_c(concat!("{\"name\":\"", $name, "\",\"status\":\"ok\"}"))
        }
    };
}

/// 生成业务函数的 FFI 入口：解析 JSON 参数、catch_unwind、序列化返回
#[macro_export]
macro_rules! hap_fn {
    ($symbol:ident, $params_ty:ty, $body:expr) => {
        #[no_mangle]
        #[allow(clippy::not_unsafe_ptr_arg_deref)]
        pub extern "C" fn $symbol(
            params_json: *const std::os::raw::c_char,
        ) -> *const std::os::raw::c_char {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let params: $params_ty = hap_common::json::parse_params(params_json)?;
                let handler: fn($params_ty) -> Result<serde_json::Value, hap_common::HapError> = $body;
                handler(params)
            }));
            match result {
                Ok(Ok(val)) => hap_common::json::ok_json(&val),
                Ok(Err(e)) => hap_common::json::error_json(&e),
                Err(_) => hap_common::json::error_json(
                    &hap_common::HapError::internal("function panicked during execution"),
                ),
            }
        }
    };
}
