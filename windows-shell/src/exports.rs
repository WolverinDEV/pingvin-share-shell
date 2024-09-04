use std::{
    ffi::c_void,
    ptr,
    sync::atomic::{AtomicUsize, Ordering},
    thread,
};

use msgbox::IconType;
use windows::Win32::{
    Foundation::{
        CLASS_E_CLASSNOTAVAILABLE, CLASS_E_NOAGGREGATION, HMODULE, STATUS_UNSUCCESSFUL, S_FALSE,
        S_OK,
    },
    System::{
        Com::IClassFactory,
        SystemServices::{
            DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH, DLL_THREAD_ATTACH, DLL_THREAD_DETACH,
        },
    },
    UI::Shell::{SHChangeNotify, SHCNE_ASSOCCHANGED, SHCNF_IDLIST},
};
use windows_core::{Interface, GUID, HRESULT};

use crate::{
    com::{IPingvinExplorerCommand, PingvinExplorerCommandHandler, SimpleClassFactory},
    config,
    installer::{legacy_ctx_menu, sparse},
    logger,
    util::{self, guid_to_string},
};

pub static mut DLL_HANDLE: HMODULE = HMODULE(ptr::null_mut());
pub static DLL_REF_COUNT: AtomicUsize = AtomicUsize::new(0);

#[no_mangle]
pub extern "system" fn DllMain(h_module: HMODULE, reason: u32, _: usize) -> i32 {
    match reason {
        DLL_PROCESS_ATTACH => {
            unsafe { DLL_HANDLE = h_module };
            if let Err(err) = logger::init() {
                let _ = msgbox::create(
                    "Pingvin Shell",
                    &format!("Failed to initialize logging:\n{}", err),
                    IconType::Error,
                );
            } else {
                log::info!("DllMain Process attach. hModule: {:X}", h_module.0 as usize);
            }

            if let Err(err) = config::init_config() {
                let _ = msgbox::create(
                    "Pingvin Shell",
                    &format!("Failed to initialize shell configuration:\n{}", err),
                    IconType::Error,
                );
            } else {
                log::debug!("Configuration loaded: {:?}", config::current_config());
            }
        }
        DLL_PROCESS_DETACH => {
            log::info!("DllMain process detach.");
        }
        DLL_THREAD_ATTACH => log::debug!("DllMain thread attach"),
        DLL_THREAD_DETACH => log::debug!("DllMain thread detach"),
        reason => log::warn!("DllMain with invalid reason {}", reason),
    };

    1
}

#[no_mangle]
pub extern "system" fn DllRegisterServer() -> HRESULT {
    log::debug!("DllRegisterServer");

    if util::is_windows_11() {
        log::info!("Register context menu handler via sparse package (win 11)");

        /* Register the sparse package. Due to WinRT requirements, this must happen in another thread */
        match thread::spawn(sparse::install).join().unwrap() {
            Ok(_) => {}
            Err(err) => {
                log::error!("Failed to register new context menu: {:?}", err);
                return STATUS_UNSUCCESSFUL.into();
            }
        };
    } else {
        log::info!("Register context menu handler via registry (pre win 11)");

        match legacy_ctx_menu::register("PingvinShare", "*", &IPingvinExplorerCommand::IID) {
            Ok(_) => {}
            Err(err) => {
                log::error!("Failed to register old context menu: {}", err);
                return STATUS_UNSUCCESSFUL.into();
            }
        };
    }

    unsafe { SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None) };

    log::info!("Context menu registered");
    S_OK
}

#[no_mangle]
pub extern "system" fn DllUnregisterServer() -> HRESULT {
    log::debug!("DllUnregisterServer");

    if let Err(err) = sparse::uninstall() {
        log::error!("Failed to uninstall sparse package: {}", err);
    }
    legacy_ctx_menu::unregister("PingvinShare", "*", &IPingvinExplorerCommand::IID);

    unsafe { SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None) };

    S_OK
}

#[no_mangle]
pub extern "system" fn DllCanUnloadNow() -> HRESULT {
    let lock_count = DLL_REF_COUNT.load(Ordering::Relaxed);
    log::debug!("DllCanUnloadNow -> {}", lock_count);
    if lock_count > 0 {
        S_FALSE
    } else {
        S_OK
    }
}

#[no_mangle]
pub extern "system" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    let rclsid = unsafe { &*rclsid };
    let riid = unsafe { &*riid };

    if *rclsid == IPingvinExplorerCommand::IID && *riid == IClassFactory::IID {
        let factory = SimpleClassFactory::new(|punkouter| {
            if punkouter.is_some() {
                return Err(CLASS_E_NOAGGREGATION.into());
            }

            Ok(PingvinExplorerCommandHandler.into())
        });
        let factory: IClassFactory = factory.into();
        unsafe { *ppv = factory.into_raw() };
        return S_OK;
    } else {
        log::warn!(
            "DllGetClassObject({}, {}) for invalid class",
            guid_to_string(rclsid),
            guid_to_string(riid)
        );
    }

    CLASS_E_CLASSNOTAVAILABLE
}
