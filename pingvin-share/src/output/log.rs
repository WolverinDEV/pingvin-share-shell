use crate::api::{PublicConfiguration, UploadEvent, UploadEventCallback};

pub fn create_log_upload_event_handler(
    server_config: &PublicConfiguration,
) -> Box<UploadEventCallback> {
    let app_url = server_config
        .get_string("general.appUrl")
        .unwrap_or("")
        .to_string();

    Box::new(move |event| match event {
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
    })
}
