use crate::api::{PublicConfiguration, UploadEvent, UploadEventCallback};

use super::AppOutput;

struct ConsoleAppOutput {}

impl AppOutput for ConsoleAppOutput {
    fn show_upload_error(&self, error: &anyhow::Error) {
        for line in format!("{:#}", error).lines() {
            log::error!("{:#}", line);
        }
    }

    fn create_upload_handler(
        &self,
        server_config: &PublicConfiguration,
    ) -> anyhow::Result<Box<UploadEventCallback>> {
        let app_url = server_config
            .get_string("general.appUrl")
            .unwrap_or("")
            .to_string();

        Ok(Box::new(move |event| match event {
            UploadEvent::ShareCreated { share_id } => {
                let share_url = format!("{}/s/{}", app_url, share_id);
                log::info!("Share has been created: {}", share_url);
            }
            UploadEvent::ShareCompleted => {
                log::info!("Upload completed");
            }
            UploadEvent::UploadError { file, error } => {
                log::error!("Failed to upload {}: {}", file.display(), error);
            }
            UploadEvent::UploadProgress(_) => {}
        }))
    }
}

pub fn create() -> anyhow::Result<Box<dyn AppOutput>> {
    Ok(Box::new(ConsoleAppOutput {}))
}
