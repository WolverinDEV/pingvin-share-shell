use crate::api::{PublicConfiguration, UploadEventCallback};
use clap::ValueEnum;

mod console;

#[cfg(target_family = "windows")]
mod win;

#[derive(ValueEnum, Clone, Debug, PartialEq, Copy)]
#[clap(rename_all = "kebab-case")]
pub enum OutputType {
    Console,

    WindowsNotification,
}

pub trait AppOutput {
    fn show_upload_error(&self, error: &anyhow::Error);
    fn create_upload_handler(
        &self,
        server_config: &PublicConfiguration,
    ) -> anyhow::Result<Box<UploadEventCallback>>;
}

pub fn create(target: OutputType) -> anyhow::Result<Box<dyn AppOutput>> {
    match target {
        OutputType::Console => console::create(),

        OutputType::WindowsNotification => {
            #[cfg(target_family = "windows")]
            return win::create();

            #[cfg(not(target_family = "windows"))]
            anyhow::bail!("output type is not supported on this platform");
        }
    }
}
