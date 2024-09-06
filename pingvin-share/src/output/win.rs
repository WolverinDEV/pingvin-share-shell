use std::sync::{Arc, Mutex};

use anyhow::Context;
use windows::{
    Data::Xml::Dom::XmlDocument,
    Foundation::Collections::IMap,
    UI::Notifications::{
        NotificationData, ToastNotification, ToastNotificationManager, ToastNotificationPriority,
        ToastNotifier,
    },
};
use windows_core::HSTRING;

use crate::api::{PublicConfiguration, UploadEvent, UploadEventCallback};

struct ProgressNotification {
    notifier: Arc<ToastNotifier>,
    notification: ToastNotification,
    notification_data: NotificationData,

    shown: bool,
}

impl ProgressNotification {
    pub fn new(notifier: Arc<ToastNotifier>, tag: &str) -> anyhow::Result<Self> {
        let xml = XmlDocument::new()?;
        xml.LoadXml(&HSTRING::from(format!(
            r#"
                <toast scenario='reminder'>
                    <visual>
                        <binding template="ToastGeneric">
                            <text>Uploading files</text>
                            <progress
                                value="{{progressValue}}"
                                valueStringOverride="{{progressValueString}}"
                                status="{{progressStatus}}"
                            />
                        </binding>
                    </visual>
                    <audio silent="true" />
                    
                    <!-- <actions>
                        <action 
                            content='Cancel' 
                            arguments='action=cancel' 
                            activationType="background"
                            afterActivationBehavior="PendingUpdate"
                        />
                    </actions> -->
                </toast>
        "#
        )))?;

        let notification = ToastNotification::CreateToastNotification(&xml)?;
        notification.SetExpiresOnReboot(true)?;
        notification.SetPriority(ToastNotificationPriority::High)?;
        notification.SetTag(&HSTRING::from(tag))?;

        let notification_data = NotificationData::new()?;
        notification.SetData(&notification_data)?;

        Ok(Self {
            notifier,
            notification,
            notification_data,
            shown: false,
        })
    }

    fn show(&mut self) -> anyhow::Result<()> {
        self.shown = true;
        Ok(self.notifier.Show(&self.notification)?)
    }

    fn hide(&mut self) -> anyhow::Result<()> {
        self.shown = false;
        Ok(self.notifier.Hide(&self.notification)?)
    }

    fn update_data(
        &mut self,
        c: impl FnOnce(&mut IMap<HSTRING, HSTRING>) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        c(&mut self.notification_data.Values()?)?;
        if self.shown {
            self.notification_data
                .SetSequenceNumber(self.notification_data.SequenceNumber()? + 1)?;

            self.notifier
                .UpdateWithTag(&self.notification_data, &self.notification.Tag()?)?;
        }
        Ok(())
    }

    pub fn set_progress(&mut self, progress: f32, message: &str) -> anyhow::Result<()> {
        self.update_data(|data| {
            data.Insert(
                &HSTRING::from("progressValue"),
                &HSTRING::from(format!("{}", progress)),
            )?;
            data.Insert(
                &HSTRING::from("progressValueString"),
                &HSTRING::from(message),
            )?;
            Ok(())
        })
    }

    pub fn set_status(&mut self, status: &str) -> anyhow::Result<()> {
        self.update_data(|data| {
            data.Insert(
                &HSTRING::from("progressStatus"),
                &HSTRING::from(format!("{}", status)),
            )?;
            Ok(())
        })
    }
}

fn show_completion_popup(
    notifier: &ToastNotifier,
    tag: &str,
    share_url: &str,
) -> anyhow::Result<()> {
    /* TODO: "Upload completed with errors:" and then a file list */
    let xml = XmlDocument::new()?;
    xml.LoadXml(&HSTRING::from(format!(
        r#"
            <toast scenario='reminder'>
                <visual>
                    <binding template="ToastGeneric">
                        <text>Upload completed</text>
                        <text>The share URL has been copied to your clipboard</text>
                    </binding>
                </visual>
                <actions>
                    <action 
                        content='Open in Browser' 
                        arguments='{}' 
                        activationType="protocol"
                    />
                </actions>
            </toast>
    "#,
        share_url
    )))?;

    let notification = ToastNotification::CreateToastNotification(&xml)?;
    notification.SetExpiresOnReboot(true)?;
    notification.SetPriority(ToastNotificationPriority::High)?;
    notification.SetTag(&HSTRING::from(tag))?;
    notification.SetSuppressPopup(false)?;
    notifier.Show(&notification)?;
    Ok(())
}

pub fn create_win_upload_event_handler(
    server_config: &PublicConfiguration,
) -> anyhow::Result<Box<UploadEventCallback>> {
    let app_url = server_config
        .get_string("general.appUrl")
        .unwrap_or("")
        .to_string();

    let notifier = Arc::new(
        Err(())
            .or_else(|_| ToastNotificationManager::CreateToastNotifier())
            .or_else(|_| {
                ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(
                    "dev.wolveringer.pingvin-share-shell",
                ))
            })
            .or_else(|_| {
                ToastNotificationManager::CreateToastNotifierWithId(
                &HSTRING::from(
            "{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\\WindowsPowerShell\\v1.0\\powershell.exe",
        ))
            })
            .context("notifier")?,
    );
    let mut progress_notification = ProgressNotification::new(notifier.clone(), "upload-progress")?;
    progress_notification.set_status("Initializing upload...")?;
    progress_notification.set_progress(0.0, "")?;
    progress_notification.show()?;

    let handler = Mutex::new({
        let mut share_url = None;
        move |event| -> anyhow::Result<()> {
            match event {
                UploadEvent::ShareCreated { share_id } => {
                    share_url = Some(format!("{}/s/{}", app_url, share_id));
                    log::info!("Share has been created: {}", share_url.as_ref().unwrap());
                    progress_notification.set_status("Uploading...")?;
                }
                UploadEvent::ShareCompleted => {
                    log::info!("Upload completed");
                    progress_notification.set_progress(1.0, "")?;
                    progress_notification.set_status("Files uploaded")?;
                    progress_notification.hide()?;

                    let share_url = share_url.as_ref().map(String::as_str).unwrap_or("");
                    show_completion_popup(
                        &*notifier,
                        &progress_notification.notification.Tag()?.to_string_lossy(),
                        share_url,
                    )?;
                    if let Err(err) = clipboard_win::set_clipboard_string(&share_url) {
                        log::warn!("Failed to copy URL to clipboard: {}", err);
                    } else {
                        log::info!("URL copied to clipboard");
                    }
                }
                UploadEvent::UploadError { file, error } => {
                    log::error!("Failed to upload {}: {}", file.display(), error);
                }
                UploadEvent::UploadProgress(progress) => {
                    let files_done = progress.files_failed + progress.files_uploaded;
                    let file_done =
                        progress.file_bytes_uploaded as f32 / progress.file_length as f32;

                    progress_notification.set_status(&format!(
                        "Uploading ({}/{})",
                        files_done, progress.files_total,
                    ))?;
                    progress_notification.set_progress(
                        (files_done as f32 + file_done) / progress.files_total as f32,
                        &format!(
                            "{}/{} bytes",
                            progress.file_bytes_uploaded, progress.file_length
                        ),
                    )?;
                }
            }
            Ok(())
        }
    });
    Ok(Box::new(move |event| {
        if let Err(err) = (*handler.lock().unwrap())(event) {
            log::warn!("Failed to process upload event: {}", err);
        }
    }))
}
