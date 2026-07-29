pub mod hap_format;
pub mod funcs;

use hap_common::{hap_free_string, hap_module_init};

hap_module_init!("app_manager");
hap_free_string!();

#[no_mangle]
pub extern "C" fn hap_module_describe() -> *const std::os::raw::c_char {
    hap_common::ffi::str_to_c(include_str!("../manifest.json"))
}
