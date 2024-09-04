use {
    anyhow::Context,
    std::{env, path::PathBuf, process::Command},
    winresource::WindowsResource,
};

fn build_sparse_package() -> anyhow::Result<PathBuf> {
    println!("cargo:rerun-if-changed=package/AppxManifest.xml");

    let package_path =
        PathBuf::from(env::var_os("OUT_DIR").context("missing out dir")?).join("package.msix");

    let mut command = Command::new("makeappx");
    command.args(&[
        "pack",
        "/d",
        "./package",
        "/p",
        &package_path.display().to_string(),
        "/nv",
        "/o",
    ]);

    let exit = command
        .spawn()
        .context("execute command makeappx")?
        .wait()?;

    if !exit.success() {
        anyhow::bail!(
            "makeappx failed with exit code 0x{:X}",
            exit.code().unwrap_or(-1)
        );
    }

    Ok(package_path)
}

fn main() -> anyhow::Result<()> {
    if env::var_os("CARGO_CFG_WINDOWS").is_none() {
        anyhow::bail!("Only Windows supported");
    }

    let sparse_package = build_sparse_package()?;
    println!(
        "cargo:rustc-env=SPARSE_PACKAGE_PATH={}",
        sparse_package.display()
    );

    /* include the icon as resource as it's used for the context menu */
    WindowsResource::new()
        .set_icon("../assets/icon.ico")
        .compile()?;
    Ok(())
}
