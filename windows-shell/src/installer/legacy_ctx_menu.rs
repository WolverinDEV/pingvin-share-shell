use windows_core::GUID;
use winreg::{enums::HKEY_LOCAL_MACHINE, RegKey};

use crate::util::{get_dll_path, guid_to_string};

pub fn register(app_id: &str, file_type: &str, clsid: *const GUID) -> anyhow::Result<()> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

    /* first we set the shell extension values */
    {
        let (shell_key, _) = hklm.create_subkey(format!(
            "Software\\Classes\\{}\\shell\\{}",
            file_type, app_id
        ))?;

        shell_key.set_value("", &format!("{} Context Menu", app_id))?;
        shell_key.set_value(
            "ExplorerCommandHandler",
            &guid_to_string(clsid).to_lowercase(),
        )?;
        shell_key.set_value("NeverDefault", &"")?;
    }

    {
        let (cls_key, _) = hklm.create_subkey(format!(
            "Software\\Classes\\CLSID\\{}",
            guid_to_string(clsid)
        ))?;
        cls_key.set_value("", &app_id)?;

        let (in_proc_key, _) = cls_key.create_subkey("InProcServer32")?;
        in_proc_key.set_value("", &format!("{}", get_dll_path().display()))?;
        in_proc_key.set_value("ThreadingModel", &"Apartment")?;
    }

    Ok(())
}

pub fn unregister(app_id: &str, file_type: &str, clsid: *const GUID) {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

    let _ = hklm.delete_subkey_all(format!(
        "Software\\Classes\\{}\\shell\\{}",
        file_type, app_id
    ));
    let _ = hklm.delete_subkey_all(format!(
        "Software\\Classes\\CLSID\\{}",
        guid_to_string(clsid)
    ));
}
