//! Session-level CGEventTap that swallows Ctrl+Space.
//!
//! The tap runs on its own thread and only signals the main thread, which
//! owns all TIS calls. A slow input-method switch therefore never stalls the
//! tap callback into a kCGEventTapDisabledByTimeout.

use std::cell::Cell;
use std::ffi::c_void;
use std::ptr;
use std::sync::mpsc;

use core_foundation::base::TCFType;
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
use core_foundation::string::{CFString, CFStringRef};
use core_foundation_sys::base::CFRelease;
use core_foundation_sys::mach_port::{CFMachPortCreateRunLoopSource, CFMachPortRef};
use core_foundation_sys::runloop::{
    CFRunLoopAddSource, CFRunLoopGetCurrent, CFRunLoopRun, kCFRunLoopCommonModes,
};

type CGEventRef = *mut c_void;
type CGEventTapProxy = *mut c_void;
type CGEventTapCallBack =
    extern "C" fn(CGEventTapProxy, u32, CGEventRef, *mut c_void) -> CGEventRef;

const SESSION_EVENT_TAP: u32 = 1;
const HEAD_INSERT_EVENT_TAP: u32 = 0;
const EVENT_TAP_OPTION_DEFAULT: u32 = 0;

const KEY_DOWN: u32 = 10;
const KEY_UP: u32 = 11;
const TAP_DISABLED_BY_TIMEOUT: u32 = 0xFFFF_FFFE;
const TAP_DISABLED_BY_USER_INPUT: u32 = 0xFFFF_FFFF;

const FIELD_AUTOREPEAT: u32 = 8;
const FIELD_KEYCODE: u32 = 9;
const KEYCODE_SPACE: i64 = 49;

const FLAG_SHIFT: u64 = 1 << 17;
const FLAG_CONTROL: u64 = 1 << 18;
const FLAG_ALTERNATE: u64 = 1 << 19;
const FLAG_COMMAND: u64 = 1 << 20;
const MODIFIERS: u64 = FLAG_SHIFT | FLAG_CONTROL | FLAG_ALTERNATE | FLAG_COMMAND;

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    static kAXTrustedCheckOptionPrompt: CFStringRef;

    fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        events_of_interest: u64,
        callback: CGEventTapCallBack,
        user_info: *mut c_void,
    ) -> CFMachPortRef;
    fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
    fn CGEventGetFlags(event: CGEventRef) -> u64;
    fn CGEventGetIntegerValueField(event: CGEventRef, field: u32) -> i64;
    fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> bool;
}

struct State {
    port: Cell<CFMachPortRef>,
    /// A Ctrl+Space keyDown was swallowed; swallow its repeats and keyUp too,
    /// so no app sees a dangling Space.
    swallowing: Cell<bool>,
    toggle: mpsc::Sender<()>,
}

extern "C" fn callback(
    _proxy: CGEventTapProxy,
    ty: u32,
    event: CGEventRef,
    user_info: *mut c_void,
) -> CGEventRef {
    let state = unsafe { &*(user_info as *const State) };
    match ty {
        TAP_DISABLED_BY_TIMEOUT | TAP_DISABLED_BY_USER_INPUT => {
            eprintln!("event tap disabled (type {ty:#x}), re-enabling");
            unsafe { CGEventTapEnable(state.port.get(), true) };
            return event;
        }
        KEY_DOWN | KEY_UP => {}
        _ => return event,
    }
    if unsafe { CGEventGetIntegerValueField(event, FIELD_KEYCODE) } != KEYCODE_SPACE {
        return event;
    }
    let swallow = if ty == KEY_UP {
        state.swallowing.replace(false)
    } else if unsafe { CGEventGetIntegerValueField(event, FIELD_AUTOREPEAT) } != 0 {
        state.swallowing.get()
    } else if unsafe { CGEventGetFlags(event) } & MODIFIERS == FLAG_CONTROL {
        state.swallowing.set(true);
        let _ = state.toggle.send(());
        true
    } else {
        false
    };
    if swallow { ptr::null_mut() } else { event }
}

/// Starts the tap thread. Each received `()` is one Ctrl+Space press.
pub fn spawn() -> Result<mpsc::Receiver<()>, String> {
    let (toggle, presses) = mpsc::channel();
    let (ready_tx, ready_rx) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let state: &'static State = Box::leak(Box::new(State {
            port: Cell::new(ptr::null_mut()),
            swallowing: Cell::new(false),
            toggle,
        }));
        let port = unsafe {
            CGEventTapCreate(
                SESSION_EVENT_TAP,
                HEAD_INSERT_EVENT_TAP,
                EVENT_TAP_OPTION_DEFAULT,
                (1 << KEY_DOWN) | (1 << KEY_UP),
                callback,
                state as *const State as *mut c_void,
            )
        };
        if port.is_null() {
            let _ = ready_tx.send(false);
            return;
        }
        state.port.set(port);
        unsafe {
            let source = CFMachPortCreateRunLoopSource(ptr::null(), port, 0);
            CFRunLoopAddSource(CFRunLoopGetCurrent(), source, kCFRunLoopCommonModes);
            CFRelease(source as *const c_void);
        }
        let _ = ready_tx.send(true);
        unsafe { CFRunLoopRun() };
    });
    if ready_rx.recv() == Ok(true) {
        return Ok(presses);
    }
    // Registers this binary in the Accessibility list and shows the prompt.
    let prompt = CFDictionary::from_CFType_pairs(&[(
        unsafe { CFString::wrap_under_get_rule(kAXTrustedCheckOptionPrompt) },
        CFBoolean::true_value(),
    )]);
    unsafe { AXIsProcessTrustedWithOptions(prompt.as_concrete_TypeRef()) };
    Err(
        "CGEventTapCreate failed: allow this binary under System Settings > \
         Privacy & Security > Accessibility, then restart it"
            .into(),
    )
}
