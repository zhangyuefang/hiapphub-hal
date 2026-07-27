pub mod client;
pub mod types;
pub mod connect;

use hap_common::ffi::str_to_c;
use std::ffi::c_char;

hap_common::hap_module_init!("automation");
hap_common::hap_free_string!();

#[no_mangle]
pub extern "C" fn hap_module_describe() -> *const c_char {
    str_to_c(include_str!("../manifest.json"))
}
