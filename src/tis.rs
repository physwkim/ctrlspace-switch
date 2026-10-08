//! Text Input Sources (Carbon HIToolbox) FFI.
//!
//! TIS is not thread safe; every call in this module must come from the
//! main thread.

use std::ffi::c_void;

use core_foundation::array::{CFArray, CFArrayRef};
use core_foundation::base::{Boolean, CFType, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
use core_foundation::string::{CFString, CFStringRef};

type TISInputSourceRef = *const c_void;

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    static kTISPropertyInputSourceID: CFStringRef;
    static kTISPropertyInputSourceCategory: CFStringRef;
    static kTISPropertyInputSourceIsSelectCapable: CFStringRef;
    static kTISCategoryKeyboardInputSource: CFStringRef;

    fn TISCopyCurrentKeyboardInputSource() -> TISInputSourceRef;
    fn TISGetInputSourceProperty(source: TISInputSourceRef, key: CFStringRef) -> *const c_void;
    fn TISCreateInputSourceList(
        properties: CFDictionaryRef,
        include_all_installed: Boolean,
    ) -> CFArrayRef;
    fn TISSelectInputSource(source: TISInputSourceRef) -> i32;
}

pub struct InputSource(CFType);

impl InputSource {
    pub fn id(&self) -> String {
        unsafe {
            let id = TISGetInputSourceProperty(self.0.as_CFTypeRef(), kTISPropertyInputSourceID);
            CFString::wrap_under_get_rule(id as CFStringRef).to_string()
        }
    }

    /// Returns the OSStatus of `TISSelectInputSource`.
    pub fn select(&self) -> i32 {
        unsafe { TISSelectInputSource(self.0.as_CFTypeRef()) }
    }
}

fn key(k: CFStringRef) -> CFString {
    unsafe { CFString::wrap_under_get_rule(k) }
}

fn list(filter: &[(CFString, CFType)]) -> Vec<InputSource> {
    let filter = CFDictionary::from_CFType_pairs(filter);
    unsafe {
        let sources = TISCreateInputSourceList(filter.as_concrete_TypeRef(), 0);
        if sources.is_null() {
            return Vec::new();
        }
        CFArray::<CFType>::wrap_under_create_rule(sources)
            .iter()
            .map(|s| InputSource(s.clone()))
            .collect()
    }
}

/// Enabled input source with exactly this `kTISPropertyInputSourceID`.
pub fn find(id: &str) -> Option<InputSource> {
    let pair = (
        key(unsafe { kTISPropertyInputSourceID }),
        CFString::new(id).as_CFType(),
    );
    list(&[pair]).into_iter().next()
}

/// IDs of every enabled, selectable keyboard input source.
pub fn enabled_ids() -> Vec<String> {
    let filter = unsafe {
        [
            (
                key(kTISPropertyInputSourceCategory),
                key(kTISCategoryKeyboardInputSource).as_CFType(),
            ),
            (
                key(kTISPropertyInputSourceIsSelectCapable),
                CFBoolean::true_value().as_CFType(),
            ),
        ]
    };
    list(&filter).iter().map(InputSource::id).collect()
}

pub fn current() -> InputSource {
    unsafe {
        InputSource(CFType::wrap_under_create_rule(
            TISCopyCurrentKeyboardInputSource(),
        ))
    }
}
