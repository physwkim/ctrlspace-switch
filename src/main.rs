//! Toggles Korean/English on Ctrl+Space via its own event tap instead of the
//! system "Select the previous input source" hotkey, whose handler reverts
//! the switch when both keys are released together (FB24297598).
//!
//! Disable that hotkey in System Settings > Keyboard > Keyboard Shortcuts >
//! Input Sources so it does not fire alongside this tool.

mod tap;
mod tis;

use std::process::exit;
use std::time::Instant;

const DEFAULT_ENGLISH: &str = "com.apple.keylayout.ABC";
const DEFAULT_KOREAN: &str = "com.apple.inputmethod.Korean.2SetKorean";

const USAGE: &str = "usage: ctrlspace-switch [--english ID] [--korean ID] | --list";

fn die(msg: &str) -> ! {
    eprintln!("ctrlspace-switch: {msg}");
    exit(1);
}

fn main() {
    let mut english = DEFAULT_ENGLISH.to_string();
    let mut korean = DEFAULT_KOREAN.to_string();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--list" => {
                tis::enabled_ids().iter().for_each(|id| println!("{id}"));
                return;
            }
            "--english" => english = args.next().unwrap_or_else(|| die(USAGE)),
            "--korean" => korean = args.next().unwrap_or_else(|| die(USAGE)),
            _ => die(USAGE),
        }
    }
    let resolve = |id: &str| {
        tis::find(id).unwrap_or_else(|| die(&format!("input source {id} not enabled (see --list)")))
    };
    let english_source = resolve(&english);
    let korean_source = resolve(&korean);

    let presses = tap::spawn().unwrap_or_else(|e| die(&e));
    eprintln!("ready: Ctrl+Space toggles {english} <-> {korean}");
    for () in presses {
        let start = Instant::now();
        let current = tis::current().id();
        let (target_id, target) = if current == korean {
            (&english, &english_source)
        } else {
            (&korean, &korean_source)
        };
        let status = target.select();
        eprintln!(
            "{current} -> {target_id}: status {status}, {:.1} ms",
            start.elapsed().as_secs_f64() * 1e3
        );
    }
}
