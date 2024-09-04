use {
    std::{env, io},
    winresource::WindowsResource,
};

fn main() -> io::Result<()> {
    if env::var_os("CARGO_CFG_WINDOWS").is_some() {
        /* include the icon as resource as it's used for the context menu */
        WindowsResource::new()
            .set_icon("../assets/icon.ico")
            .compile()?;
    }

    Ok(())
}
