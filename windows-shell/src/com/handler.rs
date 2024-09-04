use std::borrow::Cow;
use std::process::Command;
use std::sync::atomic::Ordering;
use std::thread;

use anyhow::Context;
use msgbox::IconType;
use windows::Win32::System::Com::{self};
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::Shell::{
    IEnumExplorerCommand, IExplorerCommand, IExplorerCommand_Impl, IShellItemArray,
};
use windows::{core::*, Win32::Foundation::*};

use crate::config;
use crate::exports::DLL_REF_COUNT;
use crate::util::{self, get_dll_path};

#[interface("00c77ad8-030f-4ad5-b6eb-5b231e72b2ea")]
pub unsafe trait IPingvinExplorerCommand: IExplorerCommand {}

#[implement(IPingvinExplorerCommand)]
pub struct PingvinExplorerCommandHandler;

impl IPingvinExplorerCommand_Impl for PingvinExplorerCommandHandler_Impl {}

impl IExplorerCommand_Impl for PingvinExplorerCommandHandler_Impl {
    fn GetTitle(
        &self,
        _psiitemarray: Option<&IShellItemArray>,
    ) -> windows_core::Result<windows_core::PWSTR> {
        let config = config::current_config();
        let menu_title = config
            .menu_title
            .as_ref()
            .map_or("Share via Pingvin", String::as_str);

        let mut encoded_bytes = menu_title.encode_utf16().collect::<Vec<_>>();
        encoded_bytes.push(0);
        unsafe { SHStrDupW(PWSTR::from_raw(encoded_bytes.as_mut_ptr())) }
    }

    fn GetIcon(
        &self,
        _psiitemarray: Option<&IShellItemArray>,
    ) -> windows_core::Result<windows_core::PWSTR> {
        let config = config::current_config();
        let menu_icon = config.menu_icon.as_ref().map_or_else(
            || format!("{},-32512", get_dll_path().display()).into(),
            Cow::from,
        );

        let mut encoded_bytes = menu_icon.encode_utf16().collect::<Vec<_>>();
        encoded_bytes.push(0);
        unsafe { SHStrDupW(PWSTR::from_raw(encoded_bytes.as_mut_ptr())) }
    }

    fn GetToolTip(
        &self,
        _psiitemarray: Option<&IShellItemArray>,
    ) -> windows_core::Result<windows_core::PWSTR> {
        Err(ERROR_EMPTY.into())
    }

    fn GetCanonicalName(&self) -> windows_core::Result<GUID> {
        Err(ERROR_EMPTY.into())
    }

    fn GetState(
        &self,
        _psiitemarray: Option<&IShellItemArray>,
        _foktobeslow: BOOL,
    ) -> windows_core::Result<u32> {
        Ok(0)
    }

    fn Invoke(
        &self,
        psiitemarray: Option<&IShellItemArray>,
        _pbc: Option<&Com::IBindCtx>,
    ) -> windows_core::Result<()> {
        let result: anyhow::Result<()> = (|| {
            let items = psiitemarray.context("missing item array")?;
            let files = util::get_shell_items(items)?;

            let config = config::current_config();
            let pingvin_exe = config.pingvin_exe.as_ref().map_or_else(
                || {
                    get_dll_path()
                        .parent()
                        .unwrap()
                        .join("pingvin-share.exe")
                        .into()
                },
                Cow::from,
            );

            let mut command_args: Vec<String> = vec![];
            if let Some(args) = &config.pingvin_args {
                command_args.extend(args.split(",").map(String::from));
            }
            for file in files {
                command_args.extend_from_slice(&["-f".to_string(), file.display().to_string()]);
            }

            let mut command = Command::new(pingvin_exe.display().to_string());
            command.current_dir(pingvin_exe.parent().unwrap());
            command.args(command_args.as_slice());

            log::debug!("Invoking pingvin command: {:?}", command);
            match command.spawn() {
                Ok(mut child) => {
                    DLL_REF_COUNT.fetch_add(1, Ordering::Relaxed);
                    thread::spawn(move || {
                        let Ok(exit_code) = child.wait() else { return };
                        if !exit_code.success() {
                            let _ = msgbox::create(
                                "Pingvin Share",
                                &format!(
                                    "Pingvin command executed abnormally.\nExit code {:X}",
                                    exit_code.code().unwrap_or(-1)
                                ),
                                IconType::Error,
                            );
                        }

                        DLL_REF_COUNT.fetch_sub(1, Ordering::Relaxed);
                    });
                }
                Err(err) => {
                    log::error!("Failed to run pingvin command: {}", err);
                    let _ = msgbox::create(
                        "Pingvin Share",
                        &format!("Failed to invoke pingvin:\n{}", err),
                        IconType::Error,
                    );
                }
            }
            Ok(())
        })();

        if let Err(err) = result {
            log::error!("Invoke failed: {}", err);
        }

        Ok(())
    }

    fn GetFlags(&self) -> windows_core::Result<u32> {
        Ok(0)
    }

    fn EnumSubCommands(&self) -> windows_core::Result<IEnumExplorerCommand> {
        Err(STATUS_NOT_IMPLEMENTED.into())
    }
}
