use std::ffi;
use std::sync::atomic::Ordering;

use windows::Win32::System::Com::IClassFactory;
use windows::Win32::System::Com::IClassFactory_Impl;
use windows::{core::*, Win32::Foundation::*};

use crate::exports::DLL_REF_COUNT;
use crate::util::guid_to_string;

#[implement(IClassFactory)]
pub struct SimpleClassFactory {
    constructor: Box<dyn Fn(Option<&IUnknown>) -> windows_core::Result<IUnknown>>,
}

impl SimpleClassFactory {
    pub fn new<C: 'static + Fn(Option<&IUnknown>) -> windows_core::Result<IUnknown>>(
        constructor: C,
    ) -> Self {
        Self {
            constructor: Box::new(constructor),
        }
    }
}

impl IClassFactory_Impl for SimpleClassFactory_Impl {
    fn CreateInstance(
        &self,
        punkouter: Option<&IUnknown>,
        riid: *const GUID,
        ppvobject: *mut *mut ffi::c_void,
    ) -> windows_core::Result<()> {
        log::trace!("CreateInstance {}", guid_to_string(riid));

        let result = (*self.constructor)(punkouter)?;
        unsafe { result.query(riid, ppvobject).ok() }
    }

    fn LockServer(&self, flock: BOOL) -> windows_core::Result<()> {
        let lock_count = if flock.as_bool() {
            DLL_REF_COUNT.fetch_add(1, Ordering::Relaxed)
        } else {
            DLL_REF_COUNT.fetch_sub(1, Ordering::Relaxed)
        };

        log::trace!("LockServer({}) -> {}", flock.as_bool(), lock_count);
        Ok(())
    }
}
