use std::{io::Write, os::windows::ffi::OsStrExt};

use anyhow::Context;
use windows::{
    ApplicationModel::Package,
    Foundation::Uri,
    Management::Deployment::{AddPackageOptions, PackageManager, RemovalOptions},
};
use windows_core::HSTRING;

use crate::util::get_dll_path;

const SPARSE_PACKAGE_ID: &str = "PingvinShare";
const SPARSE_PACKAGE: &[u8] = include_bytes!(env!("SPARSE_PACKAGE_PATH"));

fn find_package_by_name(name: &str) -> anyhow::Result<Option<Package>> {
    let package_manager = PackageManager::new()?;
    for package in package_manager.FindPackages()? {
        if package.Id()?.Name()?.to_string_lossy() == name {
            return Ok(Some(package));
        }
    }

    Ok(None)
}

pub fn install() -> anyhow::Result<()> {
    if find_package_by_name(SPARSE_PACKAGE_ID)?.is_some() {
        log::warn!("sparse package already installed. Uninstalling old version.");
        let _ = self::uninstall().context("uninstall old package")?;
    }

    let spackage_path = {
        let mut file = tempfile::Builder::new()
            .prefix("pingvin-share")
            .suffix(".msix")
            .tempfile()
            .context("create temp package file")?;
        file.write_all(SPARSE_PACKAGE)
            .context("write package contents")?;
        file.into_temp_path()
    };
    log::debug!(
        "Temporary sparse package located at {}",
        spackage_path.display()
    );

    let external_location_uri = {
        let buffer = get_dll_path()
            .parent()
            .context("missing dll path parent")?
            .as_os_str()
            .encode_wide()
            .collect::<Vec<_>>();

        Uri::CreateUri(&HSTRING::from_wide(&buffer)?)?
    };
    let package_uri = {
        let buffer = spackage_path.as_os_str().encode_wide().collect::<Vec<_>>();

        Uri::CreateUri(&HSTRING::from_wide(&buffer)?)?
    };

    let package_manager = PackageManager::new()?;
    let options = AddPackageOptions::new()?;
    options.SetAllowUnsigned(true)?;
    options.SetExternalLocationUri(&external_location_uri)?;

    let operation = package_manager.AddPackageByUriAsync(&package_uri, &options)?;
    let result = operation.get()?;
    result
        .ExtendedErrorCode()?
        .ok()
        .map_err(|err| anyhow::anyhow!("0x{:X} {}", err.code().0, err.message()))
        .context("AddPackageByUriAsync")?;

    let _ = spackage_path
        .close()
        .inspect_err(|err| log::warn!("Faild to remove temporary sparse package: {}", err));
    Ok(())
}

pub fn uninstall() -> anyhow::Result<()> {
    let package_manager = PackageManager::new()?;
    let Some(package) = find_package_by_name(&SPARSE_PACKAGE_ID)? else {
        return Ok(());
    };

    let operation = package_manager
        .RemovePackageWithOptionsAsync(&package.Id()?.FullName()?, RemovalOptions::None)?;
    let result = operation.get()?;
    result
        .ExtendedErrorCode()?
        .ok()
        .context("failed to remove sparse package")?;

    log::debug!("Successfully removed sparse package");
    Ok(())
}
