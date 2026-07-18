#![allow(deprecated, clippy::all)]
#[allow(unexpected_cfgs)]

use cocoa::base::{id, nil, NO, YES};
use cocoa::foundation::{NSAutoreleasePool, NSString, NSData, NSSize};
#[allow(unused_imports)]
use cocoa::appkit::{NSMenu, NSMenuItem, NSVariableStatusItemLength};
use hap_common::HapError;
use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel};
use objc::{msg_send, sel, sel_impl, class};
use serde_json::Value;
use std::sync::Once;

use crate::funcs::push_menu_event;

static REGISTER_DELEGATE: Once = Once::new();
const DELEGATE_CLASS_NAME: &str = "HapTrayMenuDelegate";

fn ensure_delegate_class() {
    REGISTER_DELEGATE.call_once(|| {
        let superclass = class!(NSObject);
        let mut decl = ClassDecl::new(DELEGATE_CLASS_NAME, superclass).unwrap();
        unsafe {
            decl.add_method(
                sel!(menuItemClicked:),
                menu_item_clicked as extern "C" fn(&Object, Sel, id),
            );
            decl.add_method(
                sel!(trayButtonClicked:),
                tray_button_clicked as extern "C" fn(&Object, Sel, id),
            );
        }
        decl.register();
    });
}

extern "C" fn tray_button_clicked(_this: &Object, _sel: Sel, _sender: id) {
    push_menu_event("__left_click__".to_string());
}

extern "C" fn menu_item_clicked(_this: &Object, _sel: Sel, sender: id) {
    unsafe {
        let tag: isize = msg_send![sender, tag];
        let item_id = format!("item_{}", tag);
        let repr: id = msg_send![sender, representedObject];
        let id_str = if repr != nil {
            let bytes: *const i8 = msg_send![repr, UTF8String];
            if !bytes.is_null() {
                std::ffi::CStr::from_ptr(bytes).to_string_lossy().to_string()
            } else { item_id }
        } else { item_id };
        push_menu_event(id_str);
    }
}

pub fn create_status_item(icon_path: &str, tooltip: &str) -> Result<*mut Object, HapError> {
    ensure_delegate_class();
    unsafe {
        let _pool = NSAutoreleasePool::new(nil);
        let status_bar: id = msg_send![class!(NSStatusBar), systemStatusBar];
        let item: id = msg_send![status_bar, statusItemWithLength: NSVariableStatusItemLength];
        let _: () = msg_send![item, retain];

        if !icon_path.is_empty() && std::path::Path::new(icon_path).exists() {
            set_icon_inner(item, icon_path);
        } else {
            set_default_icon(item);
        }

        if !tooltip.is_empty() {
            let ns_tip = NSString::alloc(nil).init_str(tooltip);
            let _: () = msg_send![item, setToolTip: ns_tip];
        }

        let delegate_class = Class::get(DELEGATE_CLASS_NAME).unwrap();
        let delegate: id = msg_send![delegate_class, alloc];
        let delegate: id = msg_send![delegate, init];
        let button: id = msg_send![item, button];
        let _: () = msg_send![button, setTarget: delegate];
        let _: () = msg_send![button, setAction: sel!(trayButtonClicked:)];

        Ok(item as *mut Object)
    }
}

unsafe fn set_default_icon(item: id) {
    let size = 18u32;
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    for y in 0..size {
        for x in 0..size {
            let idx = ((y * size + x) * 4) as usize;
            let cx = x as f32 - 9.0;
            let cy = y as f32 - 9.0;
            if cx * cx + cy * cy < 64.0 {
                rgba[idx] = 64; rgba[idx+1] = 156; rgba[idx+2] = 255; rgba[idx+3] = 255;
            }
        }
    }
    let _ns_data = NSData::dataWithBytes_length_(nil, rgba.as_ptr() as *const _, rgba.len() as u64);
    let bmp_rep: id = msg_send![class!(NSBitmapImageRep), alloc];
    let bmp_rep: id = msg_send![bmp_rep, initWithBitmapDataPlanes: std::ptr::null_mut::<*mut u8>()
        pixelsWide: size as isize
        pixelsHigh: size as isize
        bitsPerSample: 8_isize
        samplesPerPixel: 4_isize
        hasAlpha: YES
        isPlanar: NO
        colorSpaceName: NSString::alloc(nil).init_str("NSDeviceRGBColorSpace")
        bytesPerRow: (size * 4) as isize
        bitsPerPixel: 32_isize];
    let bmp_data: id = msg_send![bmp_rep, bitmapData];
    std::ptr::copy_nonoverlapping(rgba.as_ptr(), bmp_data as *mut u8, rgba.len());
    let img: id = msg_send![class!(NSImage), alloc];
    let img: id = msg_send![img, initWithSize: NSSize::new(18.0, 18.0)];
    let _: () = msg_send![img, addRepresentation: bmp_rep];
    let _: () = msg_send![img, setTemplate: YES];
    let button: id = msg_send![item, button];
    let _: () = msg_send![button, setImage: img];
}

pub fn set_icon(item: *mut Object, path: &str) -> Result<(), HapError> {
    if !std::path::Path::new(path).exists() {
        return Err(HapError::internal("icon file not found"));
    }
    unsafe { set_icon_inner(item as id, path); }
    Ok(())
}

unsafe fn set_icon_inner(item: id, path: &str) {
    let ns_path = NSString::alloc(nil).init_str(path);
    let img: id = msg_send![class!(NSImage), alloc];
    let img: id = msg_send![img, initWithContentsOfFile: ns_path];
    if img != nil {
        let _: () = msg_send![img, setSize: NSSize::new(18.0, 18.0)];
        let _: () = msg_send![img, setTemplate: YES];
        let button: id = msg_send![item, button];
        let _: () = msg_send![button, setImage: img];
    }
}

pub fn set_tooltip(item: *mut Object, tooltip: &str) {
    unsafe {
        let ns_tip = NSString::alloc(nil).init_str(tooltip);
        let _: () = msg_send![item as id, setToolTip: ns_tip];
    }
}

pub fn set_title(item: *mut Object, title: &str) {
    unsafe {
        let button: id = msg_send![item as id, button];
        let ns_title = NSString::alloc(nil).init_str(title);
        let _: () = msg_send![button, setTitle: ns_title];
    }
}

pub fn set_visible(item: *mut Object, visible: bool) {
    unsafe {
        let _: () = msg_send![item as id, setVisible: if visible { YES } else { NO }];
    }
}

pub fn set_menu(item: *mut Object, items: &[Value]) {
    ensure_delegate_class();
    unsafe {
        let _pool = NSAutoreleasePool::new(nil);
        let menu: id = msg_send![class!(NSMenu), alloc];
        let menu: id = msg_send![menu, init];

        let delegate_class = Class::get(DELEGATE_CLASS_NAME).unwrap();
        let delegate: id = msg_send![delegate_class, alloc];
        let delegate: id = msg_send![delegate, init];

        for (i, item_def) in items.iter().enumerate() {
            let label = item_def["label"].as_str().unwrap_or("");
            if label == "-" {
                let sep: id = msg_send![class!(NSMenuItem), separatorItem];
                let _: () = msg_send![menu, addItem: sep];
            } else {
                let enabled = item_def["enabled"].as_bool().unwrap_or(true);
                let user_id = item_def["id"].as_str().unwrap_or("");
                let ns_label = NSString::alloc(nil).init_str(label);
                let ns_key = NSString::alloc(nil).init_str("");
                let mi: id = msg_send![class!(NSMenuItem), alloc];
                let mi: id = msg_send![mi, initWithTitle: ns_label
                    action: sel!(menuItemClicked:)
                    keyEquivalent: ns_key];
                let _: () = msg_send![mi, setTag: i as isize];
                let _: () = msg_send![mi, setEnabled: if enabled { YES } else { NO }];
                let _: () = msg_send![mi, setTarget: delegate];
                if !user_id.is_empty() {
                    let ns_id = NSString::alloc(nil).init_str(user_id);
                    let _: () = msg_send![mi, setRepresentedObject: ns_id];
                }
                let _: () = msg_send![menu, addItem: mi];
            }
        }
        let _: () = msg_send![item as id, setMenu: menu];
    }
}

pub fn remove_status_item(item: *mut Object) {
    unsafe {
        let status_bar: id = msg_send![class!(NSStatusBar), systemStatusBar];
        let _: () = msg_send![status_bar, removeStatusItem: item as id];
        let _: () = msg_send![item as id, release];
    }
}
