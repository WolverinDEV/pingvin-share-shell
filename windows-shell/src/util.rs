use core::{ffi, slice};
use std::{ffi::OsString, os::windows::ffi::OsStringExt, path::PathBuf};

use anyhow::Context;
use windows::Win32::{
    System::{
        Com::{CoTaskMemFree, StringFromGUID2},
        LibraryLoader::GetModuleFileNameW,
    },
    UI::Shell::{IEnumShellItems, IShellItem, IShellItemArray, SIGDN_FILESYSPATH},
};
use windows_core::GUID;
use winreg::{enums::HKEY_LOCAL_MACHINE, RegKey};

use crate::exports::DLL_HANDLE;

pub fn guid_to_string(uid: *const GUID) -> String {
    let mut buffer = Vec::with_capacity(64);
    buffer.resize(64, 0);
    let length = unsafe { StringFromGUID2(uid, &mut buffer) };
    if length <= 0 {
        "".into()
    } else {
        String::from_utf16_lossy(&buffer[0..(length - 1) as usize])
    }
}

pub fn get_dll_path() -> PathBuf {
    let mut buffer = Vec::new();
    buffer.resize(1024, 0);
    let path_length = unsafe { GetModuleFileNameW(DLL_HANDLE, buffer.as_mut_slice()) as usize };

    PathBuf::from(OsString::from_wide(&buffer[0..path_length]))
}

pub fn get_shell_items(items: &IShellItemArray) -> anyhow::Result<Vec<PathBuf>> {
    let item_iter: IEnumShellItems = unsafe { items.EnumItems()? };
    let mut current_item: Option<IShellItem> = None;

    let mut result = Vec::with_capacity(unsafe { items.GetCount()? } as usize);
    let mut fetch_count = 0;
    while unsafe {
        let result = item_iter.Next(slice::from_mut(&mut current_item), Some(&mut fetch_count));
        result.is_ok() && fetch_count > 0
    } {
        let current_item = current_item.as_ref().context("missing current item")?;
        let file_path = unsafe { current_item.GetDisplayName(SIGDN_FILESYSPATH) }?;

        result.push(PathBuf::from(OsString::from_wide(unsafe {
            file_path.as_wide()
        })));

        unsafe { CoTaskMemFree(Some(file_path.as_ptr() as *const ffi::c_void)) };
    }

    Ok(result)
}

pub fn is_windows_11() -> bool {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let Ok(current_version_key) =
        hklm.open_subkey("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion")
    else {
        return false;
    };

    let Ok(build_number_str) = current_version_key.get_value::<String, _>("CurrentBuildNumber")
    else {
        return false;
    };

    build_number_str.parse::<i32>().unwrap_or(0) > 22000
}
