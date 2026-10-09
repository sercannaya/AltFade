use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use windows::{
    core::PWSTR,
    Win32::{
        Foundation::CloseHandle,
        System::Threading::{
            OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
            PROCESS_QUERY_LIMITED_INFORMATION,
        },
    },
};

/// Full executable path of a process. Only needs
/// `PROCESS_QUERY_LIMITED_INFORMATION`, which (unlike `PROCESS_VM_READ`) is
/// also granted for elevated processes and doesn't trip anti-cheat handle
/// filters.
pub(super) fn image_path(pid: u32) -> Option<String> {
    if pid == 0 {
        return None;
    }
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut size = buf.len() as u32;
        let result = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut size,
        );
        let _ = CloseHandle(handle);
        result.ok()?;
        if size == 0 {
            return None;
        }
        Some(
            OsString::from_wide(&buf[..size as usize])
                .to_string_lossy()
                .into_owned(),
        )
    }
}

/// Executable file name ("game.exe") of a process.
pub(super) fn exe_name(pid: u32) -> Option<String> {
    let path = image_path(pid)?;
    path.rsplit('\\').next().map(str::to_owned)
}
