// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
// Small platform boundary: bounded owned UTF-16 buffers are kept alive for every Win32 call.
#[cfg(not(windows))]
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(windows)]
pub fn commit(temp: &Path, path: &Path, replace: bool) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn MoveFileExW(from: *const u16, to: *const u16, flags: u32) -> i32;
    }
    let from: Vec<_> = temp.as_os_str().encode_wide().chain(Some(0)).collect();
    let to: Vec<_> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    if unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), 8 | u32::from(replace)) } == 0 {
        Err(format!(
            "Cannot commit export: {}",
            std::io::Error::last_os_error()
        ))
    } else {
        Ok(())
    }
}
#[cfg(not(windows))]
pub fn commit(temp: &Path, path: &Path, replace: bool) -> Result<(), String> {
    if replace {
        fs::rename(temp, path).map_err(|e| e.to_string())
    } else {
        fs::hard_link(temp, path).map_err(|e| e.to_string())?;
        fs::remove_file(temp).map_err(|e| e.to_string())
    }
}

#[cfg(windows)]
pub fn choose(save: bool) -> Result<Option<PathBuf>, String> {
    use std::{ffi::c_void, os::windows::ffi::OsStringExt};
    #[repr(C)]
    struct Dialog {
        size: u32,
        owner: *mut c_void,
        instance: *mut c_void,
        filter: *const u16,
        custom: *mut u16,
        max_custom: u32,
        filter_index: u32,
        file: *mut u16,
        max_file: u32,
        title_file: *mut u16,
        max_title: u32,
        initial: *const u16,
        title: *const u16,
        flags: u32,
        offset: u16,
        extension: u16,
        default_extension: *const u16,
        data: isize,
        hook: *mut c_void,
        template: *const u16,
        reserved: *mut c_void,
        reserved2: u32,
        flags_ex: u32,
    }
    #[link(name = "comdlg32")]
    unsafe extern "system" {
        fn GetOpenFileNameW(dialog: *mut Dialog) -> i32;
        fn GetSaveFileNameW(dialog: *mut Dialog) -> i32;
        fn CommDlgExtendedError() -> u32;
    }
    let mut file = vec![0u16; 32768];
    let initial = if save { "selection.schem" } else { "" };
    for (out, c) in file.iter_mut().zip(initial.encode_utf16()) {
        *out = c;
    }
    let filter: Vec<u16> = (if save {
        "Sponge schematic (*.schem)\0*.schem\0\0"
    } else {
        "Modern schematics\0*.schem;*.litematic;*.nbt\0All files\0*.*\0\0"
    })
    .encode_utf16()
    .collect();
    let title: Vec<_> = (if save {
        "Export block states only — Rusty"
    } else {
        "Load schematic — Rusty"
    })
    .encode_utf16()
    .chain(Some(0))
    .collect();
    let extension: Vec<_> = "schem".encode_utf16().chain(Some(0)).collect();
    let mut dialog: Dialog = unsafe { std::mem::zeroed() };
    dialog.size = std::mem::size_of::<Dialog>() as u32;
    dialog.filter = filter.as_ptr();
    dialog.filter_index = 1;
    dialog.file = file.as_mut_ptr();
    dialog.max_file = file.len() as u32;
    dialog.title = title.as_ptr();
    dialog.default_extension = if save {
        extension.as_ptr()
    } else {
        std::ptr::null()
    };
    // Explorer, preserve process CWD, require parent and existing input; explicit overwrite confirmation.
    dialog.flags = 0x80000 | 0x8 | 0x800 | if save { 0x2 } else { 0x1000 };
    let ok = unsafe {
        if save {
            GetSaveFileNameW(&mut dialog)
        } else {
            GetOpenFileNameW(&mut dialog)
        }
    };
    if ok == 0 {
        let error = unsafe { CommDlgExtendedError() };
        return if error == 0 {
            Ok(None)
        } else {
            Err(format!("Native file chooser failed ({error:#x})"))
        };
    }
    let len = file
        .iter()
        .position(|c| *c == 0)
        .ok_or("File path exceeds native chooser limit")?;
    Ok(Some(PathBuf::from(std::ffi::OsString::from_wide(
        &file[..len],
    ))))
}
#[cfg(not(windows))]
pub fn choose(save: bool) -> Result<Option<PathBuf>, String> {
    // Desktop-native installed dialog helpers, never Python/PowerShell or shell interpolation.
    #[cfg(target_os="macos")]
    let result = std::process::Command::new("osascript").args(["-e", if save {"POSIX path of (choose file name with prompt \"Export block states only\" default name \"selection.schem\")"} else {"POSIX path of (choose file with prompt \"Load schematic\")"}]).output();
    #[cfg(not(target_os = "macos"))]
    let result = {
        let mut command = std::process::Command::new("zenity");
        command.arg("--file-selection");
        if save {
            command.args([
                "--save",
                "--confirm-overwrite",
                "--filename=selection.schem",
            ]);
        }
        command.output()
    };
    let output = result.map_err(|_| "Native file chooser unavailable on this desktop")?;
    if !output.status.success() {
        return Ok(None);
    }
    let path =
        String::from_utf8(output.stdout).map_err(|_| "Native chooser returned invalid text")?;
    Ok(Some(PathBuf::from(path.trim_end_matches(['\r', '\n']))))
}
