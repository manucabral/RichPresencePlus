//! Running processes and the foreground window, read with Win32 calls.
//!
//! The only module with `unsafe`; each block says why it is sound.
#![allow(unsafe_code)]

use std::collections::BTreeSet;
use std::path::Path;

use serde_json::{Value, json};
use windows::Win32::Foundation::FILETIME;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Pipes::GetNamedPipeServerProcessId;
use windows::Win32::System::ProcessStatus::{
    GetProcessMemoryInfo, K32EmptyWorkingSet, PROCESS_MEMORY_COUNTERS,
};
use windows::Win32::System::SystemInformation::{GetLocalTime, GetTickCount};
use windows::Win32::System::Threading::{
    GetCurrentProcess, GetProcessTimes, OpenProcess, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};

use crate::usage::ProcessEntry;
use webview2_com::Microsoft::Web::WebView2::Win32::{
    COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL,
    ICoreWebView2_19, ICoreWebView2Controller,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
};
use windows::core::{Interface, PWSTR};

/// Closes a Win32 handle when dropped.
struct Owned(HANDLE);

impl Drop for Owned {
    fn drop(&mut self) {
        // SAFETY: the handle was returned by the API that created this guard
        // and is closed exactly once, here.
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

fn utf16(buffer: &[u16]) -> String {
    let end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..end])
}

/// File names of the running executables, unique and sorted.
pub fn process_names() -> Result<Vec<String>, String> {
    // SAFETY: the snapshot handle is owned by `Owned`; `entry` is a plain
    // struct whose size field is set as the API requires before each call.
    unsafe {
        let snapshot =
            Owned(CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).map_err(|e| e.to_string())?);
        let mut entry = PROCESSENTRY32W {
            dwSize: size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut names = BTreeSet::new();
        let mut more = Process32FirstW(snapshot.0, &mut entry).is_ok();
        while more {
            names.insert(utf16(&entry.szExeFile));
            more = Process32NextW(snapshot.0, &mut entry).is_ok();
        }
        Ok(names.into_iter().collect())
    }
}

/// Every process with its parent.
pub fn process_entries() -> Result<Vec<ProcessEntry>, String> {
    // SAFETY: as in `process_names`: the snapshot is owned by `Owned` and the
    // size field is set before each call.
    unsafe {
        let snapshot =
            Owned(CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).map_err(|e| e.to_string())?);
        let mut entry = PROCESSENTRY32W {
            dwSize: size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut entries = Vec::new();
        let mut more = Process32FirstW(snapshot.0, &mut entry).is_ok();
        while more {
            entries.push(ProcessEntry {
                pid: entry.th32ProcessID,
                parent: entry.th32ParentProcessID,
                name: utf16(&entry.szExeFile),
            });
            more = Process32NextW(snapshot.0, &mut entry).is_ok();
        }
        Ok(entries)
    }
}

/// Seconds in a `FILETIME` duration, which counts 100-nanosecond steps.
fn seconds(time: FILETIME) -> f64 {
    let ticks = (u64::from(time.dwHighDateTime) << 32) | u64::from(time.dwLowDateTime);
    ticks as f64 / 10_000_000.0
}

/// Working set in bytes and processor time in seconds of one process; `None`
/// if it cannot be opened, for example because it already ended.
pub fn process_usage(pid: u32) -> Option<(u64, f64)> {
    // SAFETY: the handle is closed by `Owned`; every out-parameter is a local
    // that lives for the call, and the counters' size is passed with them.
    unsafe {
        let process = Owned(OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?);
        let mut counters = PROCESS_MEMORY_COUNTERS {
            cb: size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
            ..Default::default()
        };
        GetProcessMemoryInfo(process.0, &mut counters, counters.cb).ok()?;
        let (mut created, mut ended, mut kernel, mut user) = Default::default();
        GetProcessTimes(process.0, &mut created, &mut ended, &mut kernel, &mut user).ok()?;
        Some((
            counters.WorkingSetSize as u64,
            seconds(kernel) + seconds(user),
        ))
    }
}

/// Executable file name of a process, when it can be queried.
fn image_name(pid: u32) -> Option<String> {
    // SAFETY: the handle is closed by `Owned`; the buffer and its length are
    // passed together, and `size` is updated by the call to the used length.
    unsafe {
        let process = Owned(OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?);
        let mut buffer = [0_u16; 1024];
        let mut size = buffer.len() as u32;
        QueryFullProcessImageNameW(
            process.0,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut size,
        )
        .ok()?;
        let full = String::from_utf16_lossy(&buffer[..size as usize]);
        Path::new(&full)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
    }
}

/// Executable name, such as `Discord.exe`, of the program serving a pipe this
/// process opened as a client; `None` if it cannot be read.
pub fn pipe_server_executable(pipe: std::os::windows::io::RawHandle) -> Option<String> {
    let mut pid = 0_u32;
    // SAFETY: the caller passes the handle of a pipe it keeps open for the
    // duration of the call; `pid` is a valid place for the answer.
    unsafe { GetNamedPipeServerProcessId(HANDLE(pipe), &mut pid) }.ok()?;
    image_name(pid)
}

/// Milliseconds between the last input and now, from two 32-bit tick counts.
fn elapsed_ticks(now: u32, last_input: u32) -> u32 {
    now.wrapping_sub(last_input)
}

/// Time since the last keyboard or mouse input in this session; `None` if
/// Windows does not answer.
pub fn idle_time() -> Option<std::time::Duration> {
    let mut info = LASTINPUTINFO {
        cbSize: size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };
    // SAFETY: `info` is a plain struct with its size field set as the API
    // requires; it lives for the whole call. `GetTickCount` takes nothing.
    let (read, now) = unsafe { (GetLastInputInfo(&mut info).as_bool(), GetTickCount()) };
    read.then(|| std::time::Duration::from_millis(elapsed_ticks(now, info.dwTime).into()))
}

/// Minutes after midnight on the computer's clock.
pub fn local_minute() -> u16 {
    // SAFETY: `GetLocalTime` takes no arguments and returns a plain struct by value.
    let now = unsafe { GetLocalTime() };
    now.wHour * 60 + now.wMinute
}

/// Sends a folder to the Recycle Bin, with no dialog.
///
/// Raises:
///     String: Windows cannot move it, for example while a program holds a file.
pub fn to_recycle_bin(folder: &std::path::Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::UI::Shell::{
        FO_DELETE, FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_NOERRORUI, FOF_SILENT, SHFILEOPSTRUCTW,
        SHFileOperationW,
    };
    use windows::core::PCWSTR;

    if !folder.is_dir() {
        return Err("The folder does not exist.".to_owned());
    }
    // The call reads a list of paths: each ends in a zero, and the list in another.
    let mut paths: Vec<u16> = folder.as_os_str().encode_wide().collect();
    paths.extend([0, 0]);
    let mut operation = SHFILEOPSTRUCTW {
        wFunc: FO_DELETE,
        pFrom: PCWSTR(paths.as_ptr()),
        fFlags: (FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_NOERRORUI | FOF_SILENT).0 as u16,
        ..Default::default()
    };
    // SAFETY: `operation` is fully initialized and `paths`, which it points to, is
    // double-terminated and outlives the call. No window handle is passed.
    let code = unsafe { SHFileOperationW(&mut operation) };
    if code != 0 || operation.fAnyOperationsAborted.as_bool() {
        return Err(format!("Windows could not move the folder (code {code})."));
    }
    Ok(())
}

/// Today on the computer's clock, as `YYYY-MM-DD`.
pub fn local_day() -> String {
    // SAFETY: `GetLocalTime` takes no arguments and returns a plain struct by value.
    let now = unsafe { GetLocalTime() };
    format!("{:04}-{:02}-{:02}", now.wYear, now.wMonth, now.wDay)
}

/// Hands the pages of this process back to Windows.
pub fn trim_memory() {
    // SAFETY: `GetCurrentProcess` returns a pseudo handle that needs no
    // closing, and the call takes nothing else.
    // A refusal only leaves the memory where it was.
    let _ = unsafe { K32EmptyWorkingSet(GetCurrentProcess()) };
}

/// Sets how much memory a web view aims for.
///
/// Args:
/// Must run on the thread that owns the web view.
///
/// Args:
///     low: `true` while nobody is looking at it.
///
/// Raises:
///     String: The runtime is too old to have this setting.
pub fn set_low_memory(controller: &ICoreWebView2Controller, low: bool) -> Result<(), String> {
    let level = if low {
        COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW
    } else {
        COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL
    };
    // SAFETY: `controller` is a live COM interface and the caller is on its
    // thread, as WebView2 requires; `cast` checks the interface exists.
    unsafe {
        let webview = controller.CoreWebView2().map_err(|e| e.to_string())?;
        let webview: ICoreWebView2_19 = webview.cast().map_err(|e| e.to_string())?;
        webview
            .SetMemoryUsageTargetLevel(level)
            .map_err(|e| e.to_string())
    }
}

/// The window the user is looking at, as `{process, title}`; `null` when there
/// is no foreground window.
pub fn foreground() -> Result<Value, String> {
    // SAFETY: the window handle is only passed back to window queries, which
    // tolerate stale handles; buffers are sized from the reported length.
    unsafe {
        let window = GetForegroundWindow();
        if window.is_invalid() {
            return Ok(Value::Null);
        }
        let mut pid = 0_u32;
        GetWindowThreadProcessId(window, Some(&mut pid));
        let length = GetWindowTextLengthW(window).max(0) as usize;
        let mut buffer = vec![0_u16; length + 1];
        let copied = GetWindowTextW(window, &mut buffer).max(0) as usize;
        Ok(json!({
            "process": image_name(pid),
            "title": String::from_utf16_lossy(&buffer[..copied]),
        }))
    }
}

#[link(name = "ntdll")]
unsafe extern "system" {
    fn NtQueryInformationProcess(
        process_handle: HANDLE,
        process_information_class: u32,
        process_information: *mut std::ffi::c_void,
        process_information_length: u32,
        return_length: *mut u32,
    ) -> i32;
}

/// `ProcessCommandLineInformation`: Windows copies the command line into the
/// caller's buffer. It needs only `PROCESS_QUERY_LIMITED_INFORMATION`, so the
/// other program's memory is never opened for reading, which anti-cheat
/// software watches for.
const PROCESS_COMMAND_LINE_INFORMATION: u32 = 60;

/// Buffer for the answer: Windows allows 32,767 characters, plus the header.
const COMMAND_LINE_BUFFER: usize = 32_768 * 2 + 64;

/// `UNICODE_STRING` as the call writes it at the start of the buffer.
#[repr(C)]
struct UnicodeString {
    length: u16,
    maximum_length: u16,
    buffer: *const u16,
}

/// Whole command line of a process, program path first; `None` if it cannot
/// be queried (it ended, or Windows refuses).
pub fn process_command_line(pid: u32) -> Option<String> {
    // `u64` elements keep the buffer aligned for the header written into it.
    let mut buffer = vec![0_u64; COMMAND_LINE_BUFFER / 8];
    let bytes = buffer.len() * 8;
    let mut returned = 0_u32;
    // SAFETY: the handle is closed by `Owned`; the buffer is valid and
    // writable for `bytes` bytes, and `returned` is a local out-parameter.
    let status = unsafe {
        let process = Owned(OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?);
        NtQueryInformationProcess(
            process.0,
            PROCESS_COMMAND_LINE_INFORMATION,
            buffer.as_mut_ptr().cast(),
            bytes as u32,
            &mut returned,
        )
    };
    if status < 0 {
        return None;
    }
    let base = buffer.as_ptr() as usize;
    // SAFETY: the call succeeded, so the buffer starts with a `UNICODE_STRING`;
    // the buffer is aligned for it and larger than it.
    let header = unsafe { &*buffer.as_ptr().cast::<UnicodeString>() };
    let start = header.buffer as usize;
    let length = usize::from(header.length);
    let end = start.checked_add(length)?;
    // The characters must lie inside our own buffer, after the header.
    let inside = start >= base + size_of::<UnicodeString>()
        && end <= base + bytes
        && start.is_multiple_of(2)
        && length.is_multiple_of(2);
    if !inside {
        return None;
    }
    // SAFETY: `[start, end)` was checked to lie inside `buffer`, which is alive
    // and not written to while this slice is used; `start` is aligned for `u16`.
    let wide = unsafe { std::slice::from_raw_parts(start as *const u16, length / 2) };
    Some(String::from_utf16_lossy(wide))
}

/// Values of the declared options on the command line of a running program.
///
/// Args:
///     name: Executable file name, matched without regard to case.
///     flags: The options the manifest declares; nothing else is returned.
///
/// Returns:
///     `{ "-h": "value", ... }`, or `null` if the program is not running or its
///     command line cannot be read.
pub fn process_args(name: &str, flags: &[String]) -> Result<Value, String> {
    let entries = process_entries()?;
    let Some(entry) = entries
        .iter()
        .find(|entry| entry.name.eq_ignore_ascii_case(name))
    else {
        return Ok(Value::Null);
    };
    let Some(line) = process_command_line(entry.pid) else {
        return Ok(Value::Null);
    };
    let args = crate::args::split_command_line(&line);
    Ok(Value::Object(crate::args::pick(&args, flags)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_this_test_process() {
        let me = std::env::current_exe().unwrap();
        let name = me.file_name().unwrap().to_string_lossy().to_lowercase();
        let names = process_names().unwrap();
        assert!(
            names.iter().any(|n| n.to_lowercase() == name),
            "{name} in {names:?}"
        );
        assert!(names.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn foreground_query_does_not_fail() {
        let answer = foreground().unwrap();
        assert!(answer.is_null() || answer["title"].is_string());
    }

    #[test]
    fn filetime_durations_read_as_seconds() {
        let one_and_a_half = FILETIME {
            dwLowDateTime: 15_000_000,
            dwHighDateTime: 0,
        };
        assert!((seconds(one_and_a_half) - 1.5).abs() < 1e-9);
        let large = FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 1,
        };
        assert!((seconds(large) - 429.4967296).abs() < 1e-6);
    }

    #[test]
    fn the_process_list_includes_this_process_and_its_parent() {
        let me = std::process::id();
        let entries = process_entries().unwrap();
        let mine = entries.iter().find(|entry| entry.pid == me).unwrap();
        assert!(mine.parent != 0);
        assert!(process_usage(me).is_some());
        // An identifier no process has gives nothing, not an error.
        assert_eq!(process_usage(u32::MAX - 3), None);
    }

    #[test]
    fn the_idle_time_survives_the_tick_counter_wrapping() {
        assert_eq!(elapsed_ticks(5_000, 2_000), 3_000);
        // Input 1 s before the counter wrapped, read 2 s after.
        assert_eq!(elapsed_ticks(2_000, u32::MAX - 999), 3_000);
    }

    #[test]
    fn the_idle_time_of_this_session_is_sane() {
        let idle = idle_time().expect("Windows answers for the current session");
        // The counter covers about 49.7 days; anything bigger is a bug.
        assert!(idle.as_millis() <= u128::from(u32::MAX));
    }

    #[test]
    fn reads_the_command_line_of_this_process() {
        let line = process_command_line(std::process::id()).expect("this process can be queried");
        let me = std::env::current_exe().unwrap();
        let name = me.file_stem().unwrap().to_string_lossy().to_lowercase();
        assert!(line.to_lowercase().contains(&name), "{line}");
        assert_eq!(process_command_line(u32::MAX - 3), None);
    }

    #[test]
    fn a_program_that_is_not_running_gives_null() {
        let flags = vec!["-h".to_owned()];
        assert_eq!(
            process_args("definitely-not-running.exe", &flags).unwrap(),
            Value::Null
        );
        // This test binary runs, so its options can be asked for; none are declared here.
        let me = std::env::current_exe().unwrap();
        let name = me.file_name().unwrap().to_string_lossy().into_owned();
        assert!(process_args(&name, &flags).unwrap().is_object());
    }
}
