use std::cell::Cell;
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

thread_local! {
    static INITIALIZED: Cell<bool> = const { Cell::new(false) };
}

/// Joins the calling thread to the multithreaded apartment, once per thread.
/// Every public audio entry point calls this because Tauri async commands run
/// on whichever tokio worker picks them up. On a thread that already joined an
/// STA (the main thread) the call fails harmlessly and the thread stays STA.
pub(super) fn ensure_initialized() {
    INITIALIZED.with(|initialized| {
        if !initialized.get() {
            unsafe {
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            }
            initialized.set(true);
        }
    });
}
