use std::os::raw::c_char;
use std::sync::OnceLock;

#[repr(C)]
pub struct HapContext {
    pub emit_callback: extern "C" fn(*const c_char, *const c_char),
    pub shell_version: *const c_char,
}

struct CtxPtr(*const HapContext);
unsafe impl Send for CtxPtr {}
unsafe impl Sync for CtxPtr {}

static CONTEXT: OnceLock<CtxPtr> = OnceLock::new();

pub fn store_context(ctx: *const HapContext) {
    let _ = CONTEXT.set(CtxPtr(ctx));
}

pub fn get_shell_version() -> String {
    if let Some(CtxPtr(ctx)) = CONTEXT.get() {
        if !ctx.is_null() {
            let ptr = unsafe { (**ctx).shell_version };
            if !ptr.is_null() {
                if let Ok(s) = unsafe { std::ffi::CStr::from_ptr(ptr) }.to_str() {
                    return s.to_string();
                }
            }
        }
    }
    "unknown".to_string()
}

pub fn emit_callback(callback_id: &str, event_json: &str) {
    if let Some(CtxPtr(ctx)) = CONTEXT.get() {
        if ctx.is_null() { return; }
        let cb_id = std::ffi::CString::new(callback_id).unwrap_or_default();
        let ev = std::ffi::CString::new(event_json).unwrap_or_default();
        unsafe { ((**ctx).emit_callback)(cb_id.as_ptr(), ev.as_ptr()) };
    }
}
