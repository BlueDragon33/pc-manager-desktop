//! Session-local, unprivileged repeated-launch activation for PC Manager.
//! The cross-process signal carries no URL arguments, paths or commands.

use std::{io, iter, ptr, thread};
use tauri::Manager;
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE, WAIT_OBJECT_0,
};
use windows_sys::Win32::System::Threading::{
    CreateEventW, CreateMutexW, SetEvent, WaitForSingleObject, INFINITE,
};

const MUTEX: &str = r"Local\BlueDragon33.PCManagerDesktop.Singleton.v1";
const EVENT: &str = r"Local\BlueDragon33.PCManagerDesktop.Activate.v1";

struct OwnedHandle(HANDLE);
// A Win32 kernel handle may be waited on and closed from another thread.
// This type owns exactly one handle and does not implement Clone.
unsafe impl Send for OwnedHandle {}
impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { CloseHandle(self.0) };
        }
    }
}

pub enum LaunchState {
    Primary(PrimaryInstance),
    Secondary,
}

pub struct PrimaryInstance {
    _mutex: OwnedHandle,
    event: OwnedHandle,
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(iter::once(0)).collect()
}

pub fn enter() -> io::Result<LaunchState> {
    enter_named(MUTEX, EVENT)
}

fn enter_named(mutex_name: &str, event_name: &str) -> io::Result<LaunchState> {
    // The event exists before the mutex becomes observable by second launches.
    let event_name = wide(event_name);
    let event = unsafe { CreateEventW(ptr::null(), 0, 0, event_name.as_ptr()) };
    if event.is_null() {
        return Err(io::Error::last_os_error());
    }
    let event = OwnedHandle(event);

    let mutex_name = wide(mutex_name);
    let mutex = unsafe { CreateMutexW(ptr::null(), 0, mutex_name.as_ptr()) };
    if mutex.is_null() {
        return Err(io::Error::last_os_error());
    }
    let exists = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
    let mutex = OwnedHandle(mutex);
    if exists {
        if unsafe { SetEvent(event.0) } == 0 {
            return Err(io::Error::last_os_error());
        }
        return Ok(LaunchState::Secondary);
    }
    Ok(LaunchState::Primary(PrimaryInstance {
        _mutex: mutex,
        event,
    }))
}

impl PrimaryInstance {
    pub fn start_listener(self, app: tauri::AppHandle) -> io::Result<()> {
        thread::Builder::new()
            .name("pcmanager-activate-window".into())
            .spawn(move || {
                let primary = self;
                loop {
                    if unsafe { WaitForSingleObject(primary.event.0, INFINITE) } != WAIT_OBJECT_0 {
                        break;
                    }
                    let main_app = app.clone();
                    let _ = app.run_on_main_thread(move || {
                        if let Some(window) = main_app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.unminimize();
                            let _ = window.set_focus();
                        }
                    });
                }
            })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);

    #[test]
    fn duplicate_launch_signals_the_primary_without_another_instance() {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let suffix = format!("{}.{}", std::process::id(), id);
        let mutex = format!(r"Local\PCManager.Test.Singleton.{suffix}");
        let event = format!(r"Local\PCManager.Test.Activate.{suffix}");
        let first = enter_named(&mutex, &event).expect("first instance");
        let LaunchState::Primary(primary) = first else {
            panic!("first process must own its named mutex");
        };
        assert!(matches!(
            enter_named(&mutex, &event).expect("second instance"),
            LaunchState::Secondary
        ));
        assert_eq!(
            unsafe { WaitForSingleObject(primary.event.0, 1_000) },
            WAIT_OBJECT_0,
            "second launch must signal primary activation"
        );
    }
}
